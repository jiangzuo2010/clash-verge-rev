use super::policy::EnterprisePolicy;
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
    let mut value = serde_yaml_ng::to_value(proxy).unwrap_or_else(|_| Value::Mapping(Mapping::new()));
    if let Value::Mapping(map) = &mut value {
        // 服务端管理响应可能带 passwordConfigured，它不是 Mihomo 节点字段。
        map.remove("passwordConfigured");
    }
    value
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
    use crate::enterprise::policy::{EnterpriseAllowRule, EnterpriseAllowRuleType, EnterprisePolicy, EnterpriseProxy};
    use serde_json::json;
    use std::collections::BTreeMap;

    fn valid_policy() -> EnterprisePolicy {
        EnterprisePolicy {
            version: "2026.05.11.1".into(),
            mode: "managed-allowlist".into(),
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

    #[test]
    fn build_managed_proxy_config_preserves_vless_reality_fields() {
        let mut policy = valid_policy();
        policy.proxy.name = "LS-Reality".into();
        policy.proxy.proxy_type = "vless".into();
        policy.proxy.server = "111.22.333.134".into();
        policy.proxy.port = 2323;
        policy
            .proxy
            .extra
            .insert("uuid".into(), json!("8a89d453-7b31-46ab-8fb3-1b0369e2c5fa"));
        policy.proxy.extra.insert("network".into(), json!("tcp"));
        policy.proxy.extra.insert("udp".into(), json!(true));
        policy.proxy.extra.insert("flow".into(), json!("xtls-rprx-vision"));
        policy.proxy.extra.insert("servername".into(), json!("www.apple.com"));
        policy.proxy.extra.insert("client-fingerprint".into(), json!("chrome"));
        policy.proxy.extra.insert(
            "reality-opts".into(),
            json!({
                "public-key": "sPHwDXwDn06gcZbO4kgpzwlSt3pjUEJqTm22GIBK3EI",
                "short-id": ""
            }),
        );

        let config = build_managed_proxy_config(&policy).expect("policy should build");
        let proxy = config
            .get("proxies")
            .and_then(Value::as_sequence)
            .and_then(|items| items.first())
            .and_then(Value::as_mapping)
            .expect("proxy should be a mapping");

        assert_eq!(proxy.get("type").and_then(Value::as_str), Some("vless"));
        assert_eq!(
            proxy.get("uuid").and_then(Value::as_str),
            Some("8a89d453-7b31-46ab-8fb3-1b0369e2c5fa")
        );
        assert_eq!(proxy.get("client-fingerprint").and_then(Value::as_str), Some("chrome"));
        assert!(proxy.get("reality-opts").and_then(Value::as_mapping).is_some());
    }
}
