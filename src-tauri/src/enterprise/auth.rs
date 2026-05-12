use super::{
    EnterprisePendingAuth, EnterpriseSession, EnterpriseState, session::hydrate_enterprise_user,
    state::EnterpriseConfig, sync::now_rfc3339,
};
use anyhow::{Context as _, Result, bail};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Duration, Utc};
use once_cell::sync::OnceCell;
use parking_lot::Mutex;
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::net::TcpListener;
use tokio::sync::oneshot;
use warp::Filter as _;

const AUTH_TTL_MINUTES: i64 = 10;
static CALLBACK_SHUTDOWN: OnceCell<Mutex<Option<oneshot::Sender<()>>>> = OnceCell::new();

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EnterpriseLoginStart {
    pub auth_url: String,
    pub state: String,
    pub redirect_uri: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EnterpriseAuthCodeRequest {
    pub code: String,
    pub state: String,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

pub async fn start_enterprise_login(open_browser: bool) -> Result<EnterpriseLoginStart> {
    let mut state = EnterpriseState::load().await;
    if !state.config.enabled {
        bail!("enterprise mode is disabled");
    }

    let auth_state = random_url_safe(32)?;
    let code_verifier = random_url_safe(64)?;
    let code_challenge = pkce_challenge(&code_verifier);
    let redirect_uri = state.config.keycloak_redirect_uri.clone();
    let auth_url = build_auth_url(&state.config, &auth_state, &code_challenge)?;

    state.session.pending_auth = Some(EnterprisePendingAuth {
        state: auth_state.clone(),
        redirect_uri: redirect_uri.clone(),
        created_at: now_rfc3339(),
        code_verifier: Some(code_verifier),
    });
    state.save().await?;
    start_callback_server(&redirect_uri)?;

    if open_browser {
        open::that(&auth_url).context("failed to open enterprise login url")?;
    }

    Ok(EnterpriseLoginStart {
        auth_url,
        state: auth_state,
        redirect_uri,
    })
}

pub async fn complete_enterprise_login(request: EnterpriseAuthCodeRequest) -> Result<EnterpriseState> {
    let mut state = EnterpriseState::load().await;
    if !state.config.enabled {
        bail!("enterprise mode is disabled");
    }

    let pending = state
        .session
        .pending_auth
        .clone()
        .context("enterprise login has not been started")?;
    if pending.state != request.state {
        bail!("enterprise login state does not match");
    }
    if is_expired(&pending.created_at)? {
        state.session.pending_auth = None;
        state.save().await?;
        bail!("enterprise login has expired");
    }

    let code_verifier = pending
        .code_verifier
        .as_deref()
        .context("enterprise login verifier is missing")?;
    let token = exchange_code_for_token(&state.config, &pending.redirect_uri, &request.code, code_verifier).await?;
    state.session = EnterpriseSession {
        authenticated: true,
        access_token: Some(token.access_token),
        refresh_token: token.refresh_token,
        access_token_expires_at: token
            .expires_in
            .map(|seconds| (Utc::now() + Duration::seconds(seconds)).to_rfc3339()),
        ..EnterpriseSession::default()
    };
    if let Err(err) = hydrate_enterprise_user(&mut state).await {
        state.clear_session();
        state.clear_policy();
        state.save().await?;
        return Err(err);
    }
    state.save().await?;

    Ok(state)
}

pub fn shutdown_enterprise_login_callback() {
    if let Some(sender) = CALLBACK_SHUTDOWN.get()
        && let Some(sender) = sender.lock().take()
    {
        sender.send(()).ok();
    }
}

fn build_auth_url(config: &EnterpriseConfig, state: &str, code_challenge: &str) -> Result<String> {
    let base = trim_url(&config.keycloak_base_url)?;
    let realm = encode(&config.keycloak_realm);
    let client_id = encode(&config.keycloak_client_id);
    let redirect_uri = encode(&config.keycloak_redirect_uri);
    let state = encode(state);
    let code_challenge = encode(code_challenge);

    Ok(format!(
        "{base}/realms/{realm}/protocol/openid-connect/auth?client_id={client_id}&redirect_uri={redirect_uri}&response_type=code&scope=openid%20profile%20email&state={state}&code_challenge={code_challenge}&code_challenge_method=S256"
    ))
}

async fn exchange_code_for_token(
    config: &EnterpriseConfig,
    redirect_uri: &str,
    code: &str,
    code_verifier: &str,
) -> Result<TokenResponse> {
    let token_url = format!(
        "{}/realms/{}/protocol/openid-connect/token",
        trim_url(&config.keycloak_base_url)?,
        encode(&config.keycloak_realm)
    );
    let client = reqwest::Client::new();
    let response = client
        .post(token_url)
        .form(&[
            ("grant_type", "authorization_code"),
            ("client_id", config.keycloak_client_id.as_str()),
            ("redirect_uri", redirect_uri),
            ("code", code),
            ("code_verifier", code_verifier),
        ])
        .send()
        .await
        .context("failed to exchange enterprise login code")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        bail!("enterprise token endpoint returned {status}: {body}");
    }

    response
        .json::<TokenResponse>()
        .await
        .context("invalid enterprise token response")
}

fn start_callback_server(redirect_uri: &str) -> Result<()> {
    shutdown_enterprise_login_callback();

    let (port, callback_path) = callback_bind_parts(redirect_uri)?;
    if TcpListener::bind(("127.0.0.1", port)).is_err() {
        bail!("enterprise login callback port {port} is already in use");
    }

    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let shutdown_store = CALLBACK_SHUTDOWN.get_or_init(|| Mutex::new(None));
    *shutdown_store.lock() = Some(shutdown_tx);

    let route = warp::path::full().and(warp::query::<CallbackQuery>()).and_then(
        move |path: warp::path::FullPath, query: CallbackQuery| {
            let callback_path = callback_path.clone();
            let path = path.as_str().to_string();
            async move { handle_callback_request(path, callback_path, query).await }
        },
    );

    tokio::spawn(async move {
        warp::serve(route)
            .bind(([127, 0, 0, 1], port))
            .await
            .graceful(async {
                shutdown_rx.await.ok();
            })
            .run()
            .await;
    });

    Ok(())
}

async fn handle_callback_request(
    path: String,
    expected_path: String,
    query: CallbackQuery,
) -> std::result::Result<warp::http::Response<String>, warp::Rejection> {
    if path != expected_path {
        return Ok(callback_response(
            warp::http::StatusCode::NOT_FOUND,
            "Enterprise login callback not found.",
        ));
    }

    let result = if let Some(error) = query.error {
        let description = query.error_description.unwrap_or_default();
        Err(anyhow::anyhow!("{error}: {description}"))
    } else {
        let code = query.code.context("missing enterprise login code");
        let state = query.state.context("missing enterprise login state");
        match (code, state) {
            (Ok(code), Ok(state)) => complete_enterprise_login(EnterpriseAuthCodeRequest { code, state })
                .await
                .map(|_| ()),
            (Err(err), _) | (_, Err(err)) => Err(err),
        }
    };

    shutdown_enterprise_login_callback();

    match result {
        Ok(()) => Ok(callback_response(
            warp::http::StatusCode::OK,
            "Enterprise login completed. You can close this window.",
        )),
        Err(err) => Ok(callback_response(
            warp::http::StatusCode::BAD_REQUEST,
            &format!("Enterprise login failed: {err}"),
        )),
    }
}

fn callback_response(status: warp::http::StatusCode, message: &str) -> warp::http::Response<String> {
    let body = format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>Enterprise Login</title></head><body>{}</body></html>",
        html_escape(message)
    );

    warp::http::Response::builder()
        .status(status)
        .header("Content-Type", "text/html; charset=utf-8")
        .body(body)
        .unwrap_or_default()
}

fn callback_bind_parts(redirect_uri: &str) -> Result<(u16, String)> {
    let url = reqwest::Url::parse(redirect_uri).context("invalid enterprise login redirect uri")?;
    if url.scheme() != "http" {
        bail!("enterprise login redirect uri must use http");
    }

    let host = url.host_str().unwrap_or_default();
    if host != "127.0.0.1" && host != "localhost" {
        bail!("enterprise login redirect uri must point to localhost");
    }

    let port = url
        .port()
        .context("enterprise login redirect uri must include a port")?;
    let path = if url.path().is_empty() { "/" } else { url.path() };
    Ok((port, path.into()))
}

fn is_expired(created_at: &str) -> Result<bool> {
    let created_at = DateTime::parse_from_rfc3339(created_at)
        .context("invalid enterprise login created_at")?
        .with_timezone(&Utc);

    Ok(Utc::now() - created_at > Duration::minutes(AUTH_TTL_MINUTES))
}

fn pkce_challenge(verifier: &str) -> String {
    let hash = Sha256::digest(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(hash)
}

fn random_url_safe(bytes_len: usize) -> Result<String> {
    let mut bytes = vec![0_u8; bytes_len];
    getrandom::fill(&mut bytes).context("failed to generate enterprise login random value")?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

fn trim_url(value: &str) -> Result<&str> {
    let value = value.trim().trim_end_matches('/');
    if value.is_empty() {
        bail!("enterprise keycloak base url is empty");
    }
    Ok(value)
}

fn encode(value: &str) -> String {
    utf8_percent_encode(value, NON_ALPHANUMERIC).to_string()
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn auth_url_contains_pkce_parameters() {
        let config = EnterpriseConfig::default();
        let url = build_auth_url(&config, "state value", "challenge").unwrap();

        assert!(url.starts_with("http://localhost:8080/realms/staff/protocol/openid-connect/auth?"));
        assert!(url.contains("client_id=company%2Dproxy%2Ddesktop"));
        assert!(url.contains("redirect_uri=http%3A%2F%2F127%2E0%2E0%2E1%3A33221%2Fauth%2Fcallback"));
        assert!(url.contains("state=state%20value"));
        assert!(url.contains("code_challenge=challenge"));
        assert!(url.contains("code_challenge_method=S256"));
    }

    #[test]
    fn pkce_challenge_is_stable() {
        let challenge = pkce_challenge("verifier");

        assert_eq!(challenge, "iMnq5o6zALKXGivsnlom_0F5_WYda32GHkxlV7mq7hQ");
    }

    #[test]
    fn callback_bind_parts_accepts_localhost_redirect_uri() {
        let (port, path) = callback_bind_parts("http://127.0.0.1:33221/auth/callback").unwrap();

        assert_eq!(port, 33221);
        assert_eq!(path, "/auth/callback");
    }

    #[test]
    fn callback_bind_parts_rejects_remote_redirect_uri() {
        let result = callback_bind_parts("https://company.example/auth/callback");

        assert!(result.is_err());
    }
}
