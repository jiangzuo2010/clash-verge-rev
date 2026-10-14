use super::EnterpriseState;
use chrono::{DateTime, Utc};

const DEFAULT_REFRESH_SECONDS: u64 = 300;
const MIN_REFRESH_SECONDS: u64 = 60;
const MAX_REFRESH_SECONDS: u64 = 3600;
// Wake shortly before the session's own refresh skew so the next sync renews the token.
const TOKEN_REFRESH_LEAD_SECONDS: i64 = 90;

/// The sync also renews the access token, and only that keeps the SSO session from idling out,
/// so it must not sleep past the token's refresh point even when the policy allows longer.
pub fn next_enterprise_sync_delay_secs(state: &EnterpriseState) -> u64 {
    let policy_delay = state
        .cached_policy
        .as_ref()
        .map(|cached| cached.policy.refresh_after_seconds)
        .unwrap_or(DEFAULT_REFRESH_SECONDS);
    let token_delay = state
        .session
        .access_token_expires_at
        .as_deref()
        .and_then(|expires_at| DateTime::parse_from_rfc3339(expires_at).ok())
        .map(|expires_at| {
            let secs = (expires_at.with_timezone(&Utc) - Utc::now()).num_seconds() - TOKEN_REFRESH_LEAD_SECONDS;
            u64::try_from(secs).unwrap_or(0)
        })
        .unwrap_or(u64::MAX);
    policy_delay
        .min(token_delay)
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
            next_enterprise_sync_delay_secs(&EnterpriseState::default()),
            DEFAULT_REFRESH_SECONDS
        );
    }

    #[test]
    fn sync_delay_does_not_outlast_the_access_token() {
        let mut state = EnterpriseState::default();
        state.session.access_token_expires_at = Some((Utc::now() + chrono::Duration::seconds(300)).to_rfc3339());

        let delay = next_enterprise_sync_delay_secs(&state);

        assert!((MIN_REFRESH_SECONDS..=210).contains(&delay), "delay was {delay}");
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
                access: Default::default(),
            },
        });

        assert_eq!(next_enterprise_sync_delay_secs(&state), MIN_REFRESH_SECONDS);

        state
            .cached_policy
            .as_mut()
            .expect("policy should exist")
            .policy
            .refresh_after_seconds = MAX_REFRESH_SECONDS + 1;

        assert_eq!(next_enterprise_sync_delay_secs(&state), MAX_REFRESH_SECONDS);
    }
}
