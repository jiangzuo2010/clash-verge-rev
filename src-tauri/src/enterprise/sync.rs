use super::{
    EnterprisePolicy, EnterpriseState,
    http::{is_loopback_enterprise_url, secure_enterprise_client, validate_enterprise_https_url},
    refresh_enterprise_session_if_needed,
};
use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead as _, KeyInit as _, Payload},
};
use anyhow::{Context as _, Result, anyhow, bail};
use base64::{Engine as _, engine::general_purpose};
use chrono::Utc;
use hmac::{Hmac, Mac as _};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest as _, Sha256};

const POLICY_PATH: &str = "/enterprise/proxy/policy";
const POLICY_CRYPTO_DEFAULT_SECRET: &str = "chineuro-enterprise-policy-transport-secret-v1";
const POLICY_CRYPTO_ALGORITHM: &str = "AES-256-GCM";
const POLICY_FORBIDDEN_CODE: &str = "403001";

/// The policy service refused this user; a cached policy must not outlive that decision.
#[derive(Debug)]
struct EnterprisePolicyRejected(String);

impl std::fmt::Display for EnterprisePolicyRejected {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "enterprise policy request rejected: {}", self.0)
    }
}

impl std::error::Error for EnterprisePolicyRejected {}

pub fn is_policy_rejection(err: &anyhow::Error) -> bool {
    err.downcast_ref::<EnterprisePolicyRejected>().is_some()
}

fn ensure_policy_status(status: reqwest::StatusCode) -> Result<()> {
    if status == reqwest::StatusCode::FORBIDDEN {
        return Err(EnterprisePolicyRejected(format!("status {status}")).into());
    }
    if !status.is_success() {
        bail!("enterprise policy request failed with status {status}");
    }
    Ok(())
}

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
    validate_enterprise_https_url(&url, "enterprise policy url")?;

    let client = secure_enterprise_client()?;
    let response = client
        .get(&url)
        .bearer_auth(access_token)
        .header("Accept", "application/json")
        .header("X-App-Code", app_code)
        .send()
        .await
        .context("failed to request enterprise policy")?;

    ensure_policy_status(response.status())?;

    let value = response
        .json::<Value>()
        .await
        .context("failed to decode enterprise policy")?;
    decode_policy_response(value, !is_loopback_enterprise_url(&url)?)
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

fn decode_policy_response(value: Value, require_encrypted: bool) -> Result<EnterprisePolicy> {
    let value = unwrap_result_payload(value)?;
    if value
        .as_object()
        .and_then(|map| map.get("encrypted"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        let envelope: EnterpriseEncryptedPolicyEnvelope =
            serde_json::from_value(value).context("failed to decode encrypted enterprise policy envelope")?;
        let decrypted = decrypt_policy_envelope(&envelope)?;
        return serde_json::from_slice(&decrypted).context("failed to decode decrypted enterprise policy");
    }
    if require_encrypted {
        bail!("enterprise policy response must be encrypted");
    }
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
        if code == Some(POLICY_FORBIDDEN_CODE) {
            return Err(EnterprisePolicyRejected(message.into()).into());
        }
        bail!("enterprise policy request failed: {message}");
    }

    map.get("data")
        .cloned()
        .ok_or_else(|| anyhow!("enterprise policy response is missing data"))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EnterpriseEncryptedPolicyEnvelope {
    encrypted: bool,
    algorithm: String,
    key_id: String,
    issued_at: String,
    nonce: String,
    ciphertext: String,
    signature: String,
}

fn decrypt_policy_envelope(envelope: &EnterpriseEncryptedPolicyEnvelope) -> Result<Vec<u8>> {
    if !envelope.encrypted {
        bail!("enterprise policy envelope is not encrypted");
    }
    if envelope.algorithm != POLICY_CRYPTO_ALGORITHM {
        bail!(
            "unsupported enterprise policy encryption algorithm: {}",
            envelope.algorithm
        );
    }

    verify_policy_signature(envelope)?;

    let nonce_bytes = general_purpose::STANDARD
        .decode(&envelope.nonce)
        .context("invalid enterprise policy nonce")?;
    let nonce =
        Nonce::try_from(nonce_bytes.as_slice()).map_err(|_| anyhow!("invalid enterprise policy nonce length"))?;
    let ciphertext = general_purpose::STANDARD
        .decode(&envelope.ciphertext)
        .context("invalid enterprise policy ciphertext")?;

    let key = derive_policy_key("aes-256-gcm");
    let cipher = Aes256Gcm::new_from_slice(&key).context("invalid enterprise policy crypto key")?;
    let aad = policy_aad(&envelope.key_id, &envelope.issued_at, &envelope.nonce);
    cipher
        .decrypt(
            &nonce,
            Payload {
                msg: &ciphertext,
                aad: aad.as_bytes(),
            },
        )
        .map_err(|_| anyhow!("failed to decrypt enterprise policy"))
}

fn verify_policy_signature(envelope: &EnterpriseEncryptedPolicyEnvelope) -> Result<()> {
    let expected = general_purpose::STANDARD
        .decode(&envelope.signature)
        .context("invalid enterprise policy signature")?;
    let payload = policy_signature_payload(
        &envelope.key_id,
        &envelope.issued_at,
        &envelope.nonce,
        &envelope.ciphertext,
    );
    let key = derive_policy_key("hmac-sha256");
    let mut mac = Hmac::<Sha256>::new_from_slice(&key).context("invalid enterprise policy signing key")?;
    mac.update(payload.as_bytes());
    mac.verify_slice(&expected)
        .map_err(|_| anyhow!("enterprise policy signature verification failed"))
}

fn derive_policy_key(usage: &str) -> [u8; 32] {
    let secret = std::env::var("ENTERPRISE_POLICY_CRYPTO_SECRET")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            option_env!("ENTERPRISE_POLICY_CRYPTO_SECRET")
                .filter(|value| !value.trim().is_empty())
                .map(ToOwned::to_owned)
        })
        .unwrap_or_else(|| POLICY_CRYPTO_DEFAULT_SECRET.into());
    Sha256::digest(format!("{secret}:{usage}").as_bytes()).into()
}

fn policy_aad(key_id: &str, issued_at: &str, nonce: &str) -> String {
    [key_id, issued_at, nonce].join(".")
}

fn policy_signature_payload(key_id: &str, issued_at: &str, nonce: &str, ciphertext: &str) -> String {
    [key_id, issued_at, nonce, ciphertext].join(".")
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
        let policy = decode_policy_response(
            json!({
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
            }),
            false,
        )
        .expect("wrapped policy should decode");

        assert_eq!(policy.version, "2026.05.11.1");
        assert_eq!(policy.allowlist.len(), 1);
    }

    #[test]
    fn decode_policy_response_accepts_plain_policy() {
        let policy = decode_policy_response(
            json!({
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
            }),
            false,
        )
        .expect("plain policy should decode");

        assert_eq!(policy.version, "2026.05.11.1");
    }

    #[test]
    fn decode_policy_response_rejects_plain_policy_when_encryption_required() {
        let result = decode_policy_response(
            json!({
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
            }),
            true,
        );

        assert!(result.is_err());
    }

    #[test]
    fn forbidden_status_is_a_policy_rejection() {
        let err = ensure_policy_status(reqwest::StatusCode::FORBIDDEN).unwrap_err();

        assert!(is_policy_rejection(&err));
    }

    #[test]
    fn server_error_status_is_not_a_policy_rejection() {
        let err = ensure_policy_status(reqwest::StatusCode::INTERNAL_SERVER_ERROR).unwrap_err();

        assert!(!is_policy_rejection(&err));
    }

    #[test]
    fn forbidden_result_code_is_a_policy_rejection() {
        let err =
            decode_policy_response(serde_json::json!({ "code": "403001", "message": "无权限访问" }), true).unwrap_err();

        assert!(is_policy_rejection(&err));
    }

    #[test]
    fn internal_error_result_code_is_not_a_policy_rejection() {
        let err = decode_policy_response(serde_json::json!({ "code": "INTERNAL_ERROR", "message": "boom" }), true)
            .unwrap_err();

        assert!(!is_policy_rejection(&err));
    }
}
