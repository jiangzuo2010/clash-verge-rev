use super::{
    EnterpriseConfig, EnterpriseSession, EnterpriseState,
    http::{secure_enterprise_client, validate_enterprise_https_url},
};
use anyhow::{Context as _, Result, anyhow, bail};
use chrono::{DateTime, Duration, Utc};
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use serde::Deserialize;
use serde_json::Value;

const IAM_ME_PATH: &str = "/iam/user/me";
const IAM_ME_APPS_PATH: &str = "/iam/user/me/apps";
const TOKEN_REFRESH_SKEW_SECONDS: i64 = 60;

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in: Option<i64>,
}

pub async fn refresh_enterprise_session_if_needed() -> Result<EnterpriseState> {
    let mut state = EnterpriseState::load().await;
    if !state.config.enabled {
        bail!("enterprise mode is disabled");
    }
    if let Err(err) = ensure_access_token(&mut state).await {
        state.save().await?;
        return Err(err);
    }
    state.save().await?;
    Ok(state)
}

pub async fn revoke_enterprise_session(state: &EnterpriseState) -> Result<()> {
    let Some(refresh_token) = state.session.refresh_token.as_deref() else {
        return Ok(());
    };

    let url = logout_url(&state.config)?;
    validate_enterprise_https_url(&url, "enterprise logout url")?;
    let response = secure_enterprise_client()?
        .post(url)
        .form(&[
            ("client_id", state.config.keycloak_client_id.as_str()),
            ("refresh_token", refresh_token),
        ])
        .send()
        .await
        .context("failed to revoke enterprise session")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        bail!("enterprise logout returned {status}: {body}");
    }

    Ok(())
}

pub async fn hydrate_enterprise_user(state: &mut EnterpriseState) -> Result<()> {
    let access_token = state
        .session
        .access_token
        .clone()
        .ok_or_else(|| anyhow!("enterprise session is missing access token"))?;
    let current_user = fetch_current_user(&state.config, &access_token).await?;
    ensure_app_authorized(&state.config, &access_token).await?;

    state.session.user_id = read_string(&current_user, "id");
    state.session.username = read_string(&current_user, "username");
    state.session.tenant_id = read_string(&current_user, "tenantId");
    state.session.tenant_code = read_string(&current_user, "tenantCode");
    state.session.is_super_admin = current_user
        .get("isSuperAdmin")
        .and_then(Value::as_bool)
        .unwrap_or_default();
    state.session.permissions = read_string_list(&current_user, "permissions", &["permissionCode", "code"]);
    state.session.roles = read_string_list(&current_user, "roles", &["roleCode", "code", "name"]);
    Ok(())
}

async fn ensure_access_token(state: &mut EnterpriseState) -> Result<()> {
    if !state.session.authenticated {
        bail!("enterprise session is not authenticated");
    }
    if state.session.access_token.is_none() {
        bail!("enterprise session is missing access token");
    }
    if !should_refresh(&state.session)? {
        return Ok(());
    }

    let refresh_token = state
        .session
        .refresh_token
        .as_deref()
        .ok_or_else(|| anyhow!("enterprise session is missing refresh token"))?;
    let token = match refresh_access_token(&state.config, refresh_token).await {
        Ok(token) => token,
        Err(err) => {
            state.clear_session();
            state.clear_policy();
            return Err(err);
        }
    };
    state.session.access_token = Some(token.access_token);
    if token.refresh_token.is_some() {
        state.session.refresh_token = token.refresh_token;
    }
    state.session.access_token_expires_at = token
        .expires_in
        .map(|seconds| (Utc::now() + Duration::seconds(seconds)).to_rfc3339());
    if let Err(err) = hydrate_enterprise_user(state).await {
        state.clear_session();
        state.clear_policy();
        return Err(err);
    }
    Ok(())
}

async fn refresh_access_token(config: &EnterpriseConfig, refresh_token: &str) -> Result<TokenResponse> {
    let url = token_url(config)?;
    validate_enterprise_https_url(&url, "enterprise token url")?;
    let client = secure_enterprise_client()?;
    let response = client
        .post(url)
        .form(&[
            ("grant_type", "refresh_token"),
            ("client_id", config.keycloak_client_id.as_str()),
            ("refresh_token", refresh_token),
        ])
        .send()
        .await
        .context("failed to refresh enterprise access token")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        bail!("enterprise token refresh returned {status}: {body}");
    }

    response
        .json::<TokenResponse>()
        .await
        .context("invalid enterprise token refresh response")
}

async fn fetch_current_user(config: &EnterpriseConfig, access_token: &str) -> Result<Value> {
    let url = iam_me_url(&config.iam_base_url)?;
    validate_enterprise_https_url(&url, "enterprise current user url")?;
    let response = secure_enterprise_client()?
        .get(url)
        .bearer_auth(access_token)
        .header("Accept", "application/json")
        .header("X-App-Code", &config.app_code)
        .send()
        .await
        .context("failed to request enterprise current user")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        bail!("enterprise current user request returned {status}: {body}");
    }

    let value = response
        .json::<Value>()
        .await
        .context("failed to decode enterprise current user")?;
    unwrap_iam_result(value)
}

async fn ensure_app_authorized(config: &EnterpriseConfig, access_token: &str) -> Result<()> {
    let apps = fetch_authorized_apps(config, access_token).await?;
    if apps.iter().any(|app| app == &config.app_code) {
        return Ok(());
    }

    bail!("enterprise user is not authorized for app {}", config.app_code)
}

async fn fetch_authorized_apps(config: &EnterpriseConfig, access_token: &str) -> Result<Vec<String>> {
    let url = iam_me_apps_url(&config.iam_base_url)?;
    validate_enterprise_https_url(&url, "enterprise authorized apps url")?;
    let response = secure_enterprise_client()?
        .get(url)
        .bearer_auth(access_token)
        .header("Accept", "application/json")
        .header("X-App-Code", &config.app_code)
        .send()
        .await
        .context("failed to request enterprise authorized apps")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        bail!("enterprise authorized apps request returned {status}: {body}");
    }

    let value = response
        .json::<Value>()
        .await
        .context("failed to decode enterprise authorized apps")?;
    let data = unwrap_iam_result(value)?;
    let Some(apps) = data.as_array() else {
        bail!("enterprise authorized apps response is not a list");
    };

    Ok(apps.iter().filter_map(Value::as_str).map(str::to_string).collect())
}

fn unwrap_iam_result(value: Value) -> Result<Value> {
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
        bail!("enterprise current user request failed: {message}");
    }

    map.get("data")
        .cloned()
        .ok_or_else(|| anyhow!("enterprise current user response is missing data"))
}

fn should_refresh(session: &EnterpriseSession) -> Result<bool> {
    let Some(expires_at) = &session.access_token_expires_at else {
        return Ok(false);
    };
    let expires_at = DateTime::parse_from_rfc3339(expires_at)
        .context("invalid enterprise access token expiry")?
        .with_timezone(&Utc);
    Ok(expires_at - Utc::now() <= Duration::seconds(TOKEN_REFRESH_SKEW_SECONDS))
}

fn iam_me_url(iam_base_url: &str) -> Result<String> {
    iam_url(iam_base_url, IAM_ME_PATH)
}

fn iam_me_apps_url(iam_base_url: &str) -> Result<String> {
    iam_url(iam_base_url, IAM_ME_APPS_PATH)
}

fn iam_url(iam_base_url: &str, path: &str) -> Result<String> {
    let base = trim_url(iam_base_url, "enterprise iam base url")?;
    if base.ends_with(path) {
        return Ok(base.into());
    }
    Ok(format!("{}{}", base.trim_end_matches('/'), path))
}

fn token_url(config: &EnterpriseConfig) -> Result<String> {
    keycloak_protocol_url(config, "token")
}

fn logout_url(config: &EnterpriseConfig) -> Result<String> {
    keycloak_protocol_url(config, "logout")
}

fn keycloak_protocol_url(config: &EnterpriseConfig, endpoint: &str) -> Result<String> {
    Ok(format!(
        "{}/realms/{}/protocol/openid-connect/{endpoint}",
        trim_url(&config.keycloak_base_url, "enterprise keycloak base url")?,
        encode(&config.keycloak_realm)
    ))
}

fn trim_url<'a>(value: &'a str, label: &str) -> Result<&'a str> {
    let value = value.trim().trim_end_matches('/');
    if value.is_empty() {
        bail!("{label} is empty");
    }
    Ok(value)
}

fn read_string(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(|v| match v {
        Value::String(text) if !text.trim().is_empty() => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        _ => None,
    })
}

fn read_string_list(value: &Value, key: &str, object_keys: &[&str]) -> Vec<String> {
    let Some(items) = value.get(key).and_then(Value::as_array) else {
        return Vec::new();
    };

    items
        .iter()
        .filter_map(|item| match item {
            Value::String(text) if !text.trim().is_empty() => Some(text.clone()),
            Value::Number(number) => Some(number.to_string()),
            Value::Object(_) => object_keys.iter().find_map(|object_key| read_string(item, object_key)),
            _ => None,
        })
        .collect()
}

fn encode(value: &str) -> String {
    utf8_percent_encode(value, NON_ALPHANUMERIC).to_string()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn iam_me_url_appends_default_path() {
        assert_eq!(
            iam_me_url("https://platform.company.example").unwrap(),
            "https://platform.company.example/iam/user/me"
        );
    }

    #[test]
    fn iam_me_url_keeps_full_endpoint() {
        assert_eq!(
            iam_me_url("https://platform.company.example/iam/user/me").unwrap(),
            "https://platform.company.example/iam/user/me"
        );
    }

    #[test]
    fn iam_me_apps_url_appends_default_path() {
        assert_eq!(
            iam_me_apps_url("https://platform.company.example").unwrap(),
            "https://platform.company.example/iam/user/me/apps"
        );
    }

    #[test]
    fn iam_me_apps_url_keeps_full_endpoint() {
        assert_eq!(
            iam_me_apps_url("https://platform.company.example/iam/user/me/apps").unwrap(),
            "https://platform.company.example/iam/user/me/apps"
        );
    }

    #[test]
    fn logout_url_uses_keycloak_realm() {
        assert_eq!(
            logout_url(&EnterpriseConfig::default()).unwrap(),
            "https://kc-t.taxspace.cn/realms/staff/protocol/openid-connect/logout"
        );
    }

    #[test]
    fn unwrap_iam_result_accepts_success_payload() {
        let data = unwrap_iam_result(json!({
            "code": "000000",
            "data": {
                "id": 7,
                "username": "alice",
                "tenantId": 3,
                "tenantCode": "taxspace",
                "permissions": [
                    "company-proxy-desktop:advanced",
                    { "permissionCode": "workspace:enterprise-proxy:manage" }
                ],
                "roles": [
                    { "roleCode": "ENTERPRISE_PROXY_ADMIN" }
                ]
            }
        }))
        .unwrap();

        assert_eq!(read_string(&data, "id").as_deref(), Some("7"));
        assert_eq!(read_string(&data, "username").as_deref(), Some("alice"));
        assert_eq!(read_string(&data, "tenantId").as_deref(), Some("3"));
        assert_eq!(
            read_string_list(&data, "permissions", &["permissionCode"]),
            vec!["company-proxy-desktop:advanced", "workspace:enterprise-proxy:manage"]
        );
        assert_eq!(
            read_string_list(&data, "roles", &["roleCode"]),
            vec!["ENTERPRISE_PROXY_ADMIN"]
        );
    }

    #[test]
    fn should_refresh_near_expiry() {
        let session = EnterpriseSession {
            access_token_expires_at: Some((Utc::now() + Duration::seconds(30)).to_rfc3339()),
            ..EnterpriseSession::default()
        };

        assert!(should_refresh(&session).unwrap());
    }
}
