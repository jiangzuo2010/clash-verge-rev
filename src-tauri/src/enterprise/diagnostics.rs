use super::{
    EnterpriseState,
    http::{secure_enterprise_client, validate_enterprise_https_url},
    refresh_enterprise_session_if_needed,
};
use crate::utils::dirs;
use anyhow::{Context as _, Result, anyhow, bail};
use serde_json::{Value, json};
use std::{
    fs::File,
    io::{Cursor, Read as _, Seek as _, SeekFrom, Write as _},
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};
use zip::{ZipWriter, write::SimpleFileOptions};

const DIAGNOSTICS_PATH: &str = "/enterprise/proxy/diagnostics";
const POLICY_PATH: &str = "/enterprise/proxy/policy";
const LOG_WINDOW: Duration = Duration::from_secs(3 * 24 * 60 * 60);
// Keeps the upload well under the gateway's request size limit.
const MAX_FILE_BYTES: u64 = 4 * 1024 * 1024;
const MAX_TOTAL_BYTES: u64 = 16 * 1024 * 1024;

/// Uploads recent client logs for the signed-in employee and returns the server's report id.
pub async fn upload_enterprise_diagnostics() -> Result<String> {
    let state = refresh_enterprise_session_if_needed().await?;
    let access_token = state
        .session
        .access_token
        .clone()
        .ok_or_else(|| anyhow!("enterprise session is missing access token"))?;
    let url = diagnostics_url(&state.config.policy_base_url)?;
    validate_enterprise_https_url(&url, "enterprise diagnostics url")?;

    let meta = diagnostics_meta(&state);
    let archive = tokio::task::spawn_blocking(move || build_archive(&meta)).await??;

    let response = secure_enterprise_client()?
        .post(&url)
        .bearer_auth(access_token)
        .header("Accept", "application/json")
        .header("Content-Type", "application/zip")
        .header("X-App-Code", state.config.app_code.trim())
        .header("X-Client-Version", env!("CARGO_PKG_VERSION"))
        .timeout(Duration::from_secs(120))
        .body(archive)
        .send()
        .await
        .context("failed to upload enterprise diagnostics")?;

    let status = response.status();
    let body = response.json::<Value>().await.ok();
    if !status.is_success() {
        let message = body.as_ref().map(server_message).unwrap_or_default();
        bail!("enterprise diagnostics upload failed with HTTP {status} {message}");
    }
    report_id(body.as_ref())
}

fn diagnostics_url(policy_base_url: &str) -> Result<String> {
    let base = policy_base_url.trim();
    if base.is_empty() {
        bail!("enterprise policy base url is empty");
    }
    let base = base.strip_suffix(POLICY_PATH).unwrap_or(base);
    Ok(format!("{}{}", base.trim_end_matches('/'), DIAGNOSTICS_PATH))
}

fn report_id(body: Option<&Value>) -> Result<String> {
    let body = body.ok_or_else(|| anyhow!("enterprise diagnostics response is not JSON"))?;
    if let Some(code) = body.get("code").and_then(Value::as_str)
        && code != "000000"
    {
        bail!("enterprise diagnostics upload failed: {}", server_message(body));
    }
    body.pointer("/data/reportId")
        .and_then(Value::as_str)
        .map(Into::into)
        .ok_or_else(|| anyhow!("enterprise diagnostics response is missing reportId"))
}

fn server_message(body: &Value) -> String {
    let code = body.get("code").and_then(Value::as_str).unwrap_or_default();
    let message = body
        .get("message")
        .or_else(|| body.get("msg"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    format!("{code} {message}").trim().into()
}

// Never include the enterprise state or Mihomo config: they hold tokens and the proxy password.
fn diagnostics_meta(state: &EnterpriseState) -> Value {
    let policy = state.cached_policy.as_ref();
    json!({
        "clientVersion": env!("CARGO_PKG_VERSION"),
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "userId": state.session.user_id,
        "username": state.session.username,
        "policyVersion": policy.map(|cached| cached.policy.version.clone()),
        "policySyncedAt": policy.map(|cached| cached.synced_at.clone()),
        "generatedAt": super::sync::now_rfc3339(),
    })
}

fn build_archive(meta: &Value) -> Result<Vec<u8>> {
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    zip.start_file("meta.json", options)?;
    zip.write_all(&serde_json::to_vec_pretty(meta)?)?;

    let mut budget = MAX_TOTAL_BYTES;
    let mut skipped = Vec::new();
    for (prefix, dir) in log_dirs() {
        for path in recent_logs(&dir) {
            if budget == 0 {
                break;
            }
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            // A log the service account wrote may be unreadable here; send the rest anyway.
            let bytes = match read_tail(&path, MAX_FILE_BYTES.min(budget)) {
                Ok(bytes) => bytes,
                Err(err) => {
                    skipped.push(format!("{prefix}/{name}: {err:#}"));
                    continue;
                }
            };
            budget -= bytes.len() as u64;
            zip.start_file(format!("{prefix}/{name}"), options)?;
            zip.write_all(&bytes)?;
        }
    }
    if !skipped.is_empty() {
        zip.start_file("skipped.txt", options)?;
        zip.write_all(skipped.join("\n").as_bytes())?;
    }

    Ok(zip.finish()?.into_inner())
}

fn log_dirs() -> Vec<(&'static str, PathBuf)> {
    [
        ("logs", dirs::app_logs_dir()),
        ("sidecar", dirs::sidecar_log_dir()),
        ("service", dirs::service_log_dir()),
    ]
    .into_iter()
    .filter_map(|(prefix, dir)| dir.ok().map(|dir| (prefix, dir)))
    .collect()
}

/// Log files changed within the window, newest first so the budget keeps the latest ones.
fn recent_logs(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let since = SystemTime::now() - LOG_WINDOW;
    let mut logs: Vec<(SystemTime, PathBuf)> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let meta = entry.metadata().ok()?;
            let modified = meta.modified().ok()?;
            (meta.is_file() && modified >= since && entry.path().extension().is_some_and(|ext| ext == "log"))
                .then(|| (modified, entry.path()))
        })
        .collect();
    logs.sort_by_key(|(modified, _)| std::cmp::Reverse(*modified));
    logs.into_iter().map(|(_, path)| path).collect()
}

fn read_tail(path: &Path, limit: u64) -> Result<Vec<u8>> {
    let mut file = File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let len = file.metadata()?.len();
    if len > limit {
        file.seek(SeekFrom::Start(len - limit))?;
    }
    let mut bytes = Vec::new();
    file.take(limit).read_to_end(&mut bytes)?;
    Ok(bytes)
}
