use super::EnterpriseState;

const DEFAULT_REFRESH_SECONDS: u64 = 300;
const MIN_REFRESH_SECONDS: u64 = 60;
const MAX_REFRESH_SECONDS: u64 = 3600;

pub fn next_policy_refresh_delay_secs(state: &EnterpriseState) -> u64 {
    state
        .cached_policy
        .as_ref()
        .map(|cached| cached.policy.refresh_after_seconds)
        .unwrap_or(DEFAULT_REFRESH_SECONDS)
        .clamp(MIN_REFRESH_SECONDS, MAX_REFRESH_SECONDS)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::enterprise::policy::{EnterpriseAllowRule, EnterpriseAllowRuleType, EnterprisePolicy, EnterpriseProxy};
    use std::collections::BTreeMap;

    #[test]
    fn refresh_delay_uses_default_without_policy() {
        assert_eq!(
            next_policy_refresh_delay_secs(&EnterpriseState::default()),
            DEFAULT_REFRESH_SECONDS
        );
    }

    #[test]
    fn refresh_delay_clamps_policy_value() {
        let mut state = EnterpriseState::default();
        state.cached_policy = Some(crate::enterprise::state::EnterpriseCachedPolicy {
            synced_at: "2026-05-11T00:00:00Z".into(),
            policy: EnterprisePolicy {
                version: "v1".into(),
                mode: "managed-allowlist".into(),
                expires_at: "2099-01-01T00:00:00Z".into(),
                refresh_after_seconds: 1,
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
                allowlist: vec![EnterpriseAllowRule {
                    rule_type: EnterpriseAllowRuleType::Domain,
                    value: "docs.company.example".into(),
                }],
            },
        });

        assert_eq!(next_policy_refresh_delay_secs(&state), MIN_REFRESH_SECONDS);

        state
            .cached_policy
            .as_mut()
            .expect("policy should exist")
            .policy
            .refresh_after_seconds = MAX_REFRESH_SECONDS + 1;

        assert_eq!(next_policy_refresh_delay_secs(&state), MAX_REFRESH_SECONDS);
    }
}
