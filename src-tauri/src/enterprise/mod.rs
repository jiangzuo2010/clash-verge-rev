pub mod auth;
pub mod config;
pub mod diagnostics;
mod http;
pub mod policy;
pub mod runtime;
pub mod scheduler;
pub mod session;
pub mod state;
pub mod sync;

pub use auth::{
    EnterpriseAuthCodeRequest, EnterpriseLoginStart, complete_enterprise_login, shutdown_enterprise_login_callback,
    start_enterprise_login,
};
pub use config::build_managed_proxy_config;
pub use diagnostics::upload_enterprise_diagnostics;
pub use policy::{EnterprisePolicy, EnterprisePolicyError};
pub use runtime::{apply_enterprise_runtime_state, ensure_enterprise_runtime_ready_for_core_start};
pub use scheduler::next_enterprise_sync_delay_secs;
pub use session::{hydrate_enterprise_user, refresh_enterprise_session_if_needed, revoke_enterprise_session};
pub use state::{
    EnterpriseConfig, EnterpriseConfigPatch, EnterprisePendingAuth, EnterpriseSession, EnterpriseState,
    EnterpriseStateView, ensure_managed_config_allowed, ensure_personal_mode, is_login_required,
};
pub use sync::sync_enterprise_policy_from_server;
