use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};

const MANAGED_ALLOWLIST_MODE: &str = "managed-allowlist";

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EnterprisePolicy {
    pub version: String,
    pub mode: String,
    pub expires_at: String,
    pub refresh_after_seconds: u64,
    pub proxy: EnterpriseProxy,
    pub allowlist: Vec<EnterpriseAllowRule>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EnterpriseProxy {
    pub name: String,
    #[serde(rename = "type")]
    pub proxy_type: String,
    pub server: String,
    pub port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct EnterpriseAllowRule {
    #[serde(rename = "type")]
    pub rule_type: EnterpriseAllowRuleType,
    pub value: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EnterpriseAllowRuleType {
    Domain,
    DomainSuffix,
    IpCidr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnterprisePolicyError {
    EmptyVersion,
    UnsupportedMode(String),
    InvalidExpiresAt(String),
    Expired,
    InvalidRefreshInterval,
    EmptyProxyName,
    EmptyProxyType,
    InvalidProxyName(String),
    EmptyProxyServer,
    InvalidProxyPort,
    EmptyAllowlist,
    InvalidAllowRule(String),
}

impl Display for EnterprisePolicyError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyVersion => f.write_str("enterprise policy version is empty"),
            Self::UnsupportedMode(mode) => write!(f, "unsupported enterprise policy mode: {mode}"),
            Self::InvalidExpiresAt(value) => write!(f, "invalid enterprise policy expiresAt: {value}"),
            Self::Expired => f.write_str("enterprise policy is expired"),
            Self::InvalidRefreshInterval => f.write_str("enterprise policy refreshAfterSeconds must be greater than 0"),
            Self::EmptyProxyName => f.write_str("enterprise proxy name is empty"),
            Self::EmptyProxyType => f.write_str("enterprise proxy type is empty"),
            Self::InvalidProxyName(name) => write!(f, "invalid enterprise proxy name: {name}"),
            Self::EmptyProxyServer => f.write_str("enterprise proxy server is empty"),
            Self::InvalidProxyPort => f.write_str("enterprise proxy port must be greater than 0"),
            Self::EmptyAllowlist => f.write_str("enterprise policy allowlist is empty"),
            Self::InvalidAllowRule(rule) => write!(f, "invalid enterprise allowlist rule: {rule}"),
        }
    }
}

impl std::error::Error for EnterprisePolicyError {}

impl EnterprisePolicy {
    pub fn validate(&self) -> Result<(), EnterprisePolicyError> {
        self.validate_at(Utc::now())
    }

    pub fn validate_at(&self, now: DateTime<Utc>) -> Result<(), EnterprisePolicyError> {
        if self.version.trim().is_empty() {
            return Err(EnterprisePolicyError::EmptyVersion);
        }
        if self.mode != MANAGED_ALLOWLIST_MODE {
            return Err(EnterprisePolicyError::UnsupportedMode(self.mode.clone()));
        }
        if self.refresh_after_seconds == 0 {
            return Err(EnterprisePolicyError::InvalidRefreshInterval);
        }

        let expires_at = DateTime::parse_from_rfc3339(&self.expires_at)
            .map_err(|_| EnterprisePolicyError::InvalidExpiresAt(self.expires_at.clone()))?
            .with_timezone(&Utc);
        if expires_at <= now {
            return Err(EnterprisePolicyError::Expired);
        }

        self.proxy.validate()?;
        if self.allowlist.is_empty() {
            return Err(EnterprisePolicyError::EmptyAllowlist);
        }
        for rule in &self.allowlist {
            rule.validate()?;
        }

        Ok(())
    }
}

impl EnterpriseProxy {
    fn validate(&self) -> Result<(), EnterprisePolicyError> {
        if self.name.trim().is_empty() {
            return Err(EnterprisePolicyError::EmptyProxyName);
        }
        if has_rule_separator(&self.name) {
            return Err(EnterprisePolicyError::InvalidProxyName(self.name.clone()));
        }
        if self.proxy_type.trim().is_empty() {
            return Err(EnterprisePolicyError::EmptyProxyType);
        }
        if self.server.trim().is_empty() {
            return Err(EnterprisePolicyError::EmptyProxyServer);
        }
        if self.port == 0 {
            return Err(EnterprisePolicyError::InvalidProxyPort);
        }
        Ok(())
    }
}

impl EnterpriseAllowRule {
    fn validate(&self) -> Result<(), EnterprisePolicyError> {
        let value = self.value.trim();
        if value.is_empty() || has_rule_separator(value) {
            return Err(EnterprisePolicyError::InvalidAllowRule(self.value.clone()));
        }

        match self.rule_type {
            EnterpriseAllowRuleType::Domain => {
                if value.starts_with('.') || value.contains('/') {
                    return Err(EnterprisePolicyError::InvalidAllowRule(self.value.clone()));
                }
            }
            EnterpriseAllowRuleType::DomainSuffix => {
                let suffix = value.strip_prefix('.').unwrap_or(value);
                if suffix.is_empty() || suffix.contains('/') {
                    return Err(EnterprisePolicyError::InvalidAllowRule(self.value.clone()));
                }
            }
            EnterpriseAllowRuleType::IpCidr => {
                if !value.contains('/') {
                    return Err(EnterprisePolicyError::InvalidAllowRule(self.value.clone()));
                }
            }
        }

        Ok(())
    }

    pub fn mihomo_rule(&self, proxy_group: &str) -> String {
        match self.rule_type {
            EnterpriseAllowRuleType::Domain => format!("DOMAIN,{},{}", self.value.trim(), proxy_group),
            EnterpriseAllowRuleType::DomainSuffix => {
                let value = self.value.trim();
                let suffix = value.strip_prefix('.').unwrap_or(value);
                format!("DOMAIN-SUFFIX,{suffix},{proxy_group}")
            }
            EnterpriseAllowRuleType::IpCidr => format!("IP-CIDR,{},{proxy_group},no-resolve", self.value.trim()),
        }
    }
}

fn has_rule_separator(value: &str) -> bool {
    value.contains(',') || value.contains('\n') || value.contains('\r')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_policy() -> EnterprisePolicy {
        EnterprisePolicy {
            version: "2026.05.11.1".into(),
            mode: MANAGED_ALLOWLIST_MODE.into(),
            expires_at: "2099-01-01T00:00:00Z".into(),
            refresh_after_seconds: 600,
            proxy: EnterpriseProxy {
                name: "company-proxy".into(),
                proxy_type: "http".into(),
                server: "proxy.company.example".into(),
                port: 443,
                tls: Some(true),
                username: None,
                password: None,
                extra: BTreeMap::new(),
            },
            allowlist: vec![
                EnterpriseAllowRule {
                    rule_type: EnterpriseAllowRuleType::Domain,
                    value: "docs.company.example".into(),
                },
                EnterpriseAllowRule {
                    rule_type: EnterpriseAllowRuleType::DomainSuffix,
                    value: ".corp.company.example".into(),
                },
                EnterpriseAllowRule {
                    rule_type: EnterpriseAllowRuleType::IpCidr,
                    value: "10.10.0.0/16".into(),
                },
            ],
        }
    }

    #[test]
    fn validate_accepts_managed_allowlist_policy() {
        assert_eq!(valid_policy().validate(), Ok(()));
    }

    #[test]
    fn validate_rejects_expired_policy() {
        let mut policy = valid_policy();
        policy.expires_at = "2020-01-01T00:00:00Z".into();

        assert_eq!(policy.validate(), Err(EnterprisePolicyError::Expired));
    }

    #[test]
    fn validate_rejects_path_like_domain_rule() {
        let mut policy = valid_policy();
        policy.allowlist[0].value = "docs.company.example/path".into();

        assert_eq!(
            policy.validate(),
            Err(EnterprisePolicyError::InvalidAllowRule(
                "docs.company.example/path".into()
            ))
        );
    }

    #[test]
    fn validate_rejects_zero_proxy_port() {
        let mut policy = valid_policy();
        policy.proxy.port = 0;

        assert_eq!(policy.validate(), Err(EnterprisePolicyError::InvalidProxyPort));
    }

    #[test]
    fn domain_suffix_rule_strips_leading_dot() {
        let rule = EnterpriseAllowRule {
            rule_type: EnterpriseAllowRuleType::DomainSuffix,
            value: ".corp.company.example".into(),
        };

        assert_eq!(
            rule.mihomo_rule("COMPANY-PROXY"),
            "DOMAIN-SUFFIX,corp.company.example,COMPANY-PROXY"
        );
    }
}
