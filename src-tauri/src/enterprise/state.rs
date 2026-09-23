use super::policy::{EnterpriseAllowRuleType, EnterprisePolicy};
use crate::{
    config::{deserialize_encrypted, serialize_encrypted},
    utils::{dirs, help},
};
use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const ENTERPRISE_STATE_FILE: &str = "enterprise.yaml";

#[derive(Default, Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EnterpriseState {
    #[serde(default)]
    pub config: EnterpriseConfig,
    #[serde(default)]
    pub session: EnterpriseSession,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_policy: Option<EnterpriseCachedPolicy>,
}

#[derive(Default, Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EnterpriseStateView {
    pub config: EnterpriseConfig,
    pub session: EnterpriseSessionView,
    #[serde(default)]
    pub advanced_access: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy_status: Option<EnterprisePolicyStatus>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_resources: Vec<EnterpriseAllowedResource>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EnterpriseConfig {
    pub enabled: bool,
    pub iam_base_url: String,
    pub policy_base_url: String,
    pub keycloak_base_url: String,
    pub keycloak_realm: String,
    pub keycloak_client_id: String,
    pub keycloak_redirect_uri: String,
    pub app_code: String,
}

#[derive(Default, Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EnterpriseConfigPatch {
    pub enabled: Option<bool>,
    pub iam_base_url: Option<String>,
    pub policy_base_url: Option<String>,
    pub keycloak_base_url: Option<String>,
    pub keycloak_realm: Option<String>,
    pub keycloak_client_id: Option<String>,
    pub keycloak_redirect_uri: Option<String>,
    pub app_code: Option<String>,
}

#[derive(Default, Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EnterpriseSession {
    pub authenticated: bool,
    pub is_super_admin: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub permissions: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_token_expires_at: Option<String>,
    #[serde(
        serialize_with = "serialize_encrypted",
        deserialize_with = "deserialize_encrypted",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub access_token: Option<String>,
    #[serde(
        serialize_with = "serialize_encrypted",
        deserialize_with = "deserialize_encrypted",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub refresh_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending_auth: Option<EnterprisePendingAuth>,
}

#[derive(Default, Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EnterpriseSessionView {
    pub authenticated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_token_expires_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EnterprisePendingAuth {
    pub state: String,
    pub redirect_uri: String,
    pub created_at: String,
    #[serde(
        serialize_with = "serialize_encrypted",
        deserialize_with = "deserialize_encrypted",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub code_verifier: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EnterpriseCachedPolicy {
    pub synced_at: String,
    pub policy: EnterprisePolicy,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EnterprisePolicyStatus {
    pub version: String,
    pub mode: String,
    pub expires_at: String,
    pub synced_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EnterpriseAllowedResource {
    #[serde(rename = "type")]
    pub rule_type: String,
    pub value: String,
}

impl Default for EnterpriseConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            iam_base_url: "https://api-t.taxspace.cn/gateway/api".into(),
            policy_base_url: "https://api-t.taxspace.cn/gateway/api".into(),
            keycloak_base_url: "https://kc-t.taxspace.cn".into(),
            keycloak_realm: "staff".into(),
            keycloak_client_id: "usp-enterprise-proxy".into(),
            keycloak_redirect_uri: "http://127.0.0.1:33221/auth/callback".into(),
            app_code: "company-proxy-desktop".into(),
        }
    }
}

impl EnterpriseConfig {
    pub fn patch(&mut self, patch: EnterpriseConfigPatch) {
        if let Some(enabled) = patch.enabled {
            self.enabled = enabled;
        }
        if let Some(iam_base_url) = non_empty(patch.iam_base_url) {
            self.iam_base_url = iam_base_url;
        }
        if let Some(policy_base_url) = non_empty(patch.policy_base_url) {
            self.policy_base_url = policy_base_url;
        }
        if let Some(keycloak_base_url) = non_empty(patch.keycloak_base_url) {
            self.keycloak_base_url = keycloak_base_url;
        }
        if let Some(keycloak_realm) = non_empty(patch.keycloak_realm) {
            self.keycloak_realm = keycloak_realm;
        }
        if let Some(keycloak_client_id) = non_empty(patch.keycloak_client_id) {
            self.keycloak_client_id = keycloak_client_id;
        }
        if let Some(keycloak_redirect_uri) = non_empty(patch.keycloak_redirect_uri) {
            self.keycloak_redirect_uri = keycloak_redirect_uri;
        }
        if let Some(app_code) = non_empty(patch.app_code) {
            self.app_code = app_code;
        }
    }
}

impl EnterpriseState {
    pub async fn load() -> Self {
        let path = match enterprise_state_path() {
            Ok(path) => path,
            Err(_) => return Self::default(),
        };

        help::read_yaml::<Self>(&path).await.unwrap_or_default()
    }

    pub async fn save(&self) -> Result<()> {
        let path = enterprise_state_path()?;
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .with_context(|| format!("failed to create enterprise state directory \"{}\"", parent.display()))?;
        }
        help::save_yaml(&path, self, Some("# Clash Verge Enterprise State")).await
    }

    pub const fn is_authenticated(&self) -> bool {
        self.config.enabled && self.session.authenticated && self.session.access_token.is_some()
    }

    pub fn clear_session(&mut self) {
        self.session = EnterpriseSession::default();
    }

    pub fn clear_policy(&mut self) {
        self.cached_policy = None;
    }

    pub fn has_valid_cached_policy(&self) -> bool {
        self.cached_policy
            .as_ref()
            .is_some_and(|cached| cached.policy.validate().is_ok())
    }

    pub fn has_advanced_access(&self) -> bool {
        self.is_authenticated()
            && self.has_valid_cached_policy()
            && self
                .cached_policy
                .as_ref()
                .is_some_and(|cached| cached.policy.access.advanced)
    }

    pub fn set_cached_policy(&mut self, policy: EnterprisePolicy) -> Result<()> {
        policy.validate()?;
        self.cached_policy = Some(EnterpriseCachedPolicy {
            synced_at: super::sync::now_rfc3339(),
            policy,
        });
        Ok(())
    }

    pub fn to_view(&self) -> EnterpriseStateView {
        EnterpriseStateView {
            config: self.config.clone(),
            session: EnterpriseSessionView {
                authenticated: self.session.authenticated,
                user_id: self.session.user_id.clone(),
                username: self.session.username.clone(),
                tenant_id: self.session.tenant_id.clone(),
                tenant_code: self.session.tenant_code.clone(),
                access_token_expires_at: self.session.access_token_expires_at.clone(),
            },
            advanced_access: self.has_advanced_access(),
            policy_status: self.cached_policy.as_ref().map(|cached| EnterprisePolicyStatus {
                version: cached.policy.version.clone(),
                mode: cached.policy.mode.clone(),
                expires_at: cached.policy.expires_at.clone(),
                synced_at: cached.synced_at.clone(),
            }),
            allowed_resources: self
                .cached_policy
                .as_ref()
                .map(|cached| {
                    cached
                        .policy
                        .allowlist
                        .iter()
                        .map(|rule| EnterpriseAllowedResource {
                            rule_type: match rule.rule_type {
                                EnterpriseAllowRuleType::Domain => "domain",
                                EnterpriseAllowRuleType::DomainSuffix => "domain_suffix",
                                EnterpriseAllowRuleType::IpCidr => "ip_cidr",
                            }
                            .into(),
                            value: rule.value.clone(),
                        })
                        .collect()
                })
                .unwrap_or_default(),
        }
    }
}

pub async fn ensure_personal_mode(operation: &str) -> Result<()> {
    let state = EnterpriseState::load().await;
    if state.config.enabled {
        anyhow::bail!("{operation} is disabled in enterprise managed mode");
    }
    Ok(())
}

pub fn enterprise_state_path() -> Result<PathBuf> {
    Ok(dirs::app_home_dir()?.join(ENTERPRISE_STATE_FILE))
}

fn non_empty(value: Option<String>) -> Option<String> {
    value.and_then(|v| {
        let trimmed = v.trim();
        if trimmed.is_empty() { None } else { Some(trimmed.into()) }
    })
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;

    #[test]
    fn default_enterprise_config_points_to_test_environment() {
        let config = EnterpriseConfig::default();

        assert!(config.enabled);
        assert_eq!(config.iam_base_url, "https://api-t.taxspace.cn/gateway/api");
        assert_eq!(config.policy_base_url, "https://api-t.taxspace.cn/gateway/api");
        assert_eq!(config.keycloak_base_url, "https://kc-t.taxspace.cn");
        assert_eq!(config.keycloak_realm, "staff");
        assert_eq!(config.keycloak_client_id, "usp-enterprise-proxy");
        assert_eq!(config.app_code, "company-proxy-desktop");
    }

    #[test]
    fn is_authenticated_requires_enabled_state_and_access_token() {
        let mut state = EnterpriseState::default();
        state.config.enabled = true;
        state.session.authenticated = true;

        assert!(!state.is_authenticated());

        state.session.access_token = Some("token".into());

        assert!(state.is_authenticated());
    }

    #[test]
    fn clear_session_removes_tokens() {
        let mut state = EnterpriseState::default();
        state.session.authenticated = true;
        state.session.access_token = Some("access".into());
        state.session.refresh_token = Some("refresh".into());

        state.clear_session();

        assert_eq!(state.session, EnterpriseSession::default());
    }

    #[test]
    fn state_view_does_not_expose_tokens() {
        let mut state = EnterpriseState::default();
        state.session.authenticated = true;
        state.session.username = Some("alice".into());
        state.session.access_token = Some("access".into());
        state.session.refresh_token = Some("refresh".into());

        let view = state.to_view();

        assert!(view.session.authenticated);
        assert_eq!(view.session.username.as_deref(), Some("alice"));
    }

    #[test]
    fn state_view_exposes_allowed_resources_without_proxy_details() {
        let mut state = EnterpriseState::default();
        state.cached_policy = Some(EnterpriseCachedPolicy {
            synced_at: "2026-05-11T00:00:00Z".into(),
            policy: EnterprisePolicy {
                version: "v1".into(),
                mode: "managed-allowlist".into(),
                expires_at: "2099-01-01T00:00:00Z".into(),
                refresh_after_seconds: 600,
                proxy: super::super::policy::EnterpriseProxy {
                    name: "company-proxy".into(),
                    proxy_type: "http".into(),
                    server: "proxy.company.example".into(),
                    port: 443,
                    tls: Some(true),
                    username: None,
                    password: None,
                    extra: std::collections::BTreeMap::new(),
                },
                allowlist: vec![super::super::policy::EnterpriseAllowRule {
                    rule_type: super::super::policy::EnterpriseAllowRuleType::DomainSuffix,
                    value: ".corp.company.example".into(),
                }],
                access: Default::default(),
            },
        });

        let view = state.to_view();

        assert_eq!(view.allowed_resources.len(), 1);
        assert_eq!(view.allowed_resources[0].rule_type, "domain_suffix");
        assert_eq!(view.allowed_resources[0].value, ".corp.company.example");
    }

    #[test]
    fn config_patch_ignores_empty_strings() {
        let mut config = EnterpriseConfig::default();

        config.patch(EnterpriseConfigPatch {
            keycloak_realm: Some("".into()),
            app_code: Some("company-proxy".into()),
            ..EnterpriseConfigPatch::default()
        });

        assert_eq!(config.keycloak_realm, "staff");
        assert_eq!(config.app_code, "company-proxy");
    }

    #[test]
    fn set_cached_policy_rejects_invalid_policy() {
        let mut state = EnterpriseState::default();
        let policy = EnterprisePolicy {
            version: "".into(),
            mode: "managed-allowlist".into(),
            expires_at: "2099-01-01T00:00:00Z".into(),
            refresh_after_seconds: 600,
            proxy: super::super::policy::EnterpriseProxy {
                name: "company-proxy".into(),
                proxy_type: "http".into(),
                server: "proxy.company.example".into(),
                port: 443,
                tls: Some(true),
                username: None,
                password: None,
                extra: std::collections::BTreeMap::new(),
            },
            allowlist: vec![super::super::policy::EnterpriseAllowRule {
                rule_type: super::super::policy::EnterpriseAllowRuleType::Domain,
                value: "docs.company.example".into(),
            }],
            access: Default::default(),
        };

        assert!(state.set_cached_policy(policy).is_err());
        assert!(state.cached_policy.is_none());
    }

    #[test]
    fn has_valid_cached_policy_rejects_expired_cached_policy() {
        let mut state = EnterpriseState::default();
        state.cached_policy = Some(EnterpriseCachedPolicy {
            synced_at: "2026-05-11T00:00:00Z".into(),
            policy: EnterprisePolicy {
                version: "v1".into(),
                mode: "managed-allowlist".into(),
                expires_at: "2000-01-01T00:00:00Z".into(),
                refresh_after_seconds: 600,
                proxy: super::super::policy::EnterpriseProxy {
                    name: "company-proxy".into(),
                    proxy_type: "http".into(),
                    server: "proxy.company.example".into(),
                    port: 443,
                    tls: Some(true),
                    username: None,
                    password: None,
                    extra: std::collections::BTreeMap::new(),
                },
                allowlist: vec![super::super::policy::EnterpriseAllowRule {
                    rule_type: super::super::policy::EnterpriseAllowRuleType::Domain,
                    value: "docs.company.example".into(),
                }],
                access: Default::default(),
            },
        });

        assert!(!state.has_valid_cached_policy());
    }

    fn authenticated_state() -> EnterpriseState {
        let mut state = EnterpriseState::default();
        state.session.authenticated = true;
        state.session.access_token = Some("access".into());
        state
    }

    fn cached_policy(expires_at: &str, advanced: bool) -> EnterpriseCachedPolicy {
        EnterpriseCachedPolicy {
            synced_at: "2026-05-11T00:00:00Z".into(),
            policy: EnterprisePolicy {
                version: "v1".into(),
                mode: "managed-allowlist".into(),
                expires_at: expires_at.into(),
                refresh_after_seconds: 600,
                proxy: super::super::policy::EnterpriseProxy {
                    name: "company-proxy".into(),
                    proxy_type: "http".into(),
                    server: "proxy.company.example".into(),
                    port: 443,
                    tls: Some(true),
                    username: None,
                    password: None,
                    extra: std::collections::BTreeMap::new(),
                },
                allowlist: vec![super::super::policy::EnterpriseAllowRule {
                    rule_type: super::super::policy::EnterpriseAllowRuleType::Domain,
                    value: "docs.company.example".into(),
                }],
                access: super::super::policy::EnterprisePolicyAccess { advanced },
            },
        }
    }

    #[test]
    fn advanced_access_comes_from_valid_policy() {
        let mut state = authenticated_state();
        state.cached_policy = Some(cached_policy("2099-01-01T00:00:00Z", true));

        assert!(state.to_view().advanced_access);
    }

    #[test]
    fn advanced_access_is_false_when_policy_does_not_grant_it() {
        let mut state = authenticated_state();
        state.cached_policy = Some(cached_policy("2099-01-01T00:00:00Z", false));

        assert!(!state.to_view().advanced_access);
    }

    #[test]
    fn advanced_access_is_false_for_expired_policy() {
        let mut state = authenticated_state();
        state.cached_policy = Some(cached_policy("2000-01-01T00:00:00Z", true));

        assert!(!state.to_view().advanced_access);
    }

    #[test]
    fn advanced_access_is_false_after_logout() {
        let mut state = authenticated_state();
        state.cached_policy = Some(cached_policy("2099-01-01T00:00:00Z", true));
        state.clear_session();

        assert!(!state.to_view().advanced_access);
    }

    #[test]
    fn advanced_access_is_false_after_policy_is_cleared() {
        let mut state = authenticated_state();
        state.cached_policy = Some(cached_policy("2099-01-01T00:00:00Z", true));
        state.clear_policy();

        assert!(!state.to_view().advanced_access);
    }
}
