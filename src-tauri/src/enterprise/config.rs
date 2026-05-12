use super::policy::{EnterprisePolicy, EnterpriseProxyType};
use anyhow::Result;
use serde_yaml_ng::{Mapping, Value};

const COMPANY_PROXY_GROUP: &str = "COMPANY-PROXY";

pub fn build_managed_proxy_config(policy: &EnterprisePolicy) -> Result<Mapping> {
    policy.validate()?;

    let mut config = Mapping::new();
    config.insert("mode".into(), "rule".into());
    config.insert("proxies".into(), vec![proxy_value(policy)].into());
    config.insert("proxy-groups".into(), vec![proxy_group_value(policy)].into());
    config.insert("rules".into(), rules_value(policy));
    Ok(config)
}

fn proxy_value(policy: &EnterprisePolicy) -> Value {
    let proxy = &policy.proxy;
    let mut value = Mapping::new();
    value.insert("name".into(), proxy.name.clone().into());
    value.insert(
        "type".into(),
        match proxy.proxy_type {
            EnterpriseProxyType::Http => "http",
            EnterpriseProxyType::Socks5 => "socks5",
        }
        .into(),
    );
    value.insert("server".into(), proxy.server.clone().into());
    value.insert("port".into(), proxy.port.into());
    if proxy.tls {
        value.insert("tls".into(), true.into());
    }
    if let Some(username) = proxy.username.as_ref().filter(|v| !v.trim().is_empty()) {
        value.insert("username".into(), username.clone().into());
    }
    if let Some(password) = proxy.password.as_ref().filter(|v| !v.trim().is_empty()) {
        value.insert("password".into(), password.clone().into());
    }
    value.into()
}

fn proxy_group_value(policy: &EnterprisePolicy) -> Value {
    let mut value = Mapping::new();
    value.insert("name".into(), COMPANY_PROXY_GROUP.into());
    value.insert("type".into(), "select".into());
    value.insert("proxies".into(), vec![policy.proxy.name.clone()].into());
    value.into()
}

fn rules_value(policy: &EnterprisePolicy) -> Value {
    let mut rules = policy
        .allowlist
        .iter()
        .map(|rule| Value::String(rule.mihomo_rule(COMPANY_PROXY_GROUP)))
        .collect::<Vec<_>>();
    rules.push("MATCH,DIRECT".into());
    rules.into()
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::enterprise::policy::{
        EnterpriseAllowRule, EnterpriseAllowRuleType, EnterprisePolicy, EnterpriseProxy, EnterpriseProxyType,
    };

    fn valid_policy() -> EnterprisePolicy {
        EnterprisePolicy {
            version: "2026.05.11.1".into(),
            mode: "managed-allowlist".into(),
            expires_at: "2099-01-01T00:00:00Z".into(),
            refresh_after_seconds: 600,
            proxy: EnterpriseProxy {
                name: "company-proxy".into(),
                proxy_type: EnterpriseProxyType::Http,
                server: "proxy.company.example".into(),
                port: 443,
                tls: true,
                username: None,
                password: None,
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
            ],
        }
    }

    #[test]
    fn build_managed_proxy_config_generates_direct_fallback() {
        let config = build_managed_proxy_config(&valid_policy()).expect("policy should build");
        let rules = config
            .get("rules")
            .and_then(Value::as_sequence)
            .expect("rules should be a sequence");

        assert_eq!(rules.last().and_then(Value::as_str), Some("MATCH,DIRECT"));
    }

    #[test]
    fn build_managed_proxy_config_generates_company_proxy_group() {
        let config = build_managed_proxy_config(&valid_policy()).expect("policy should build");
        let groups = config
            .get("proxy-groups")
            .and_then(Value::as_sequence)
            .expect("proxy-groups should be a sequence");
        let group = groups
            .first()
            .and_then(Value::as_mapping)
            .expect("group should be mapping");

        assert_eq!(group.get("name").and_then(Value::as_str), Some(COMPANY_PROXY_GROUP));
        assert_eq!(group.get("type").and_then(Value::as_str), Some("select"));
    }
}
