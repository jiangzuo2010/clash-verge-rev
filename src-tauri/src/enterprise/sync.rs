use super::{EnterprisePolicy, EnterpriseState, refresh_enterprise_session_if_needed};
use anyhow::{Context as _, Result, anyhow, bail};
use chrono::Utc;
use serde_json::Value;

const POLICY_PATH: &str = "/enterprise/proxy/policy";

pub async fn sync_enterprise_policy_from_server() -> Result<EnterpriseState> {
    let mut state = refresh_enterprise_session_if_needed().await?;
    let access_token = state
        .session
        .access_token
        .clone()
        .ok_or_else(|| anyhow!("enterprise session is missing access token"))?;

    let policy = fetch_policy(&state.config.policy_base_url, &access_token, &state.config.app_code).await?;
    state.set_cached_policy(policy)?;
    state.save().await?;
    Ok(state)
}

async fn fetch_policy(policy_base_url: &str, access_token: &str, app_code: &str) -> Result<EnterprisePolicy> {
    let url = policy_url(policy_base_url)?;
    let app_code = app_code.trim();
    if app_code.is_empty() {
        bail!("enterprise app code is empty");
    }

    let client = reqwest::Client::builder().build()?;
    let response = client
        .get(url)
        .bearer_auth(access_token)
        .header("Accept", "application/json")
        .header("X-App-Code", app_code)
        .send()
        .await
        .context("failed to request enterprise policy")?;

    let status = response.status();
    if !status.is_success() {
        bail!("enterprise policy request failed with status {status}");
    }

    let value = response
        .json::<Value>()
        .await
        .context("failed to decode enterprise policy")?;
    decode_policy_response(value)
}

fn policy_url(policy_base_url: &str) -> Result<String> {
    let base = policy_base_url.trim();
    if base.is_empty() {
        bail!("enterprise policy base url is empty");
    }
    if base.ends_with(POLICY_PATH) {
        return Ok(base.into());
    }
    Ok(format!("{}{}", base.trim_end_matches('/'), POLICY_PATH))
}

pub fn now_rfc3339() -> String {
    Utc::now().to_rfc3339()
}

fn decode_policy_response(value: Value) -> Result<EnterprisePolicy> {
    let value = unwrap_result_payload(value)?;
    serde_json::from_value(value).context("failed to decode enterprise policy")
}

fn unwrap_result_payload(value: Value) -> Result<Value> {
    let Some(map) = value.as_object() else {
        return Ok(value);
    };

    let code = map.get("code").and_then(Value::as_str);
    if code.is_none() {
        return Ok(value);
    }

    if code != Some("000000") {
        let message = map
            .get("message")
            .or_else(|| map.get("msg"))
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        bail!("enterprise policy request failed: {message}");
    }

    map.get("data")
        .cloned()
        .ok_or_else(|| anyhow!("enterprise policy response is missing data"))
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn policy_url_appends_default_path() {
        assert_eq!(
            policy_url("https://platform.company.example").expect("url should build"),
            "https://platform.company.example/enterprise/proxy/policy"
        );
    }

    #[test]
    fn policy_url_keeps_full_policy_endpoint() {
        assert_eq!(
            policy_url("https://platform.company.example/enterprise/proxy/policy").expect("url should build"),
            "https://platform.company.example/enterprise/proxy/policy"
        );
    }

    #[test]
    fn policy_url_rejects_empty_base() {
        assert!(policy_url(" ").is_err());
    }

    #[test]
    fn decode_policy_response_accepts_result_wrapper() {
        let policy = decode_policy_response(json!({
            "code": "000000",
            "message": "success",
            "data": {
                "version": "2026.05.11.1",
                "mode": "managed-allowlist",
                "expiresAt": "2099-01-01T00:00:00Z",
                "refreshAfterSeconds": 300,
                "proxy": {
                    "name": "company-proxy",
                    "type": "http",
                    "server": "proxy.company.example",
                    "port": 443,
                    "tls": true
                },
                "allowlist": [
                    { "type": "domain", "value": "docs.company.example" }
                ]
            }
        }))
        .expect("wrapped policy should decode");

        assert_eq!(policy.version, "2026.05.11.1");
        assert_eq!(policy.allowlist.len(), 1);
    }

    #[test]
    fn decode_policy_response_accepts_plain_policy() {
        let policy = decode_policy_response(json!({
            "version": "2026.05.11.1",
            "mode": "managed-allowlist",
            "expiresAt": "2099-01-01T00:00:00Z",
            "refreshAfterSeconds": 300,
            "proxy": {
                "name": "company-proxy",
                "type": "http",
                "server": "proxy.company.example",
                "port": 443,
                "tls": true
            },
            "allowlist": [
                { "type": "domain", "value": "docs.company.example" }
            ]
        }))
        .expect("plain policy should decode");

        assert_eq!(policy.version, "2026.05.11.1");
    }
}
