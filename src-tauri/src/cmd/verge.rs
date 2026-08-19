use super::{CmdResult, proxy_aware_error};
use crate::{
    cmd::StringifyErr as _,
    config::IVerge,
    core::notification::{self, FailedOperation},
    enterprise::EnterpriseState,
    feat,
};
use clash_verge_draft::SharedDraft;

/// 获取Verge配置
#[tauri::command]
pub async fn get_verge_config() -> CmdResult<SharedDraft<IVerge>> {
    feat::fetch_verge_config().await.stringify_err()
}

/// 修改Verge配置
#[tauri::command]
pub async fn patch_verge_config(payload: IVerge) -> CmdResult {
    ensure_enterprise_verge_patch_allowed(&payload).await.stringify_err()?;

    let operation = system_proxy_operation(&payload);
    let result = match operation {
        Some(operation) => notification::asking_for(operation, Box::pin(feat::patch_verge(&payload, false))).await,
        None => feat::patch_verge(&payload, false).await,
    };
    result.map_err(|error| proxy_aware_error(&error).asking_for(operation))
}

/// Extract a system proxy operation from a Verge patch.
const fn system_proxy_operation(payload: &IVerge) -> Option<FailedOperation> {
    match payload.enable_system_proxy {
        Some(true) => Some(FailedOperation::SystemProxyEnable),
        Some(false) => Some(FailedOperation::SystemProxyDisable),
        None => None,
    }
}

async fn ensure_enterprise_verge_patch_allowed(payload: &IVerge) -> anyhow::Result<()> {
    if !EnterpriseState::load().await.config.enabled {
        return Ok(());
    }

    if payload.enable_tun_mode.is_some()
        || payload.enable_system_proxy.is_some()
        || payload.enable_proxy_guard.is_some()
        || payload.enable_dns_settings.is_some()
        || payload.proxy_auto_config.is_some()
        || payload.proxy_host.is_some()
        || payload.use_default_bypass.is_some()
        || payload.system_proxy_bypass.is_some()
        || payload.proxy_guard_duration.is_some()
        || payload.verge_mixed_port.is_some()
        || payload.verge_socks_port.is_some()
        || payload.verge_socks_enabled.is_some()
        || payload.verge_port.is_some()
        || payload.verge_http_enabled.is_some()
        || payload.enable_external_controller.is_some()
    {
        anyhow::bail!("verge proxy settings are disabled in enterprise managed mode");
    }

    #[cfg(not(target_os = "windows"))]
    if payload.verge_redir_port.is_some() || payload.verge_redir_enabled.is_some() {
        anyhow::bail!("verge proxy settings are disabled in enterprise managed mode");
    }

    #[cfg(target_os = "linux")]
    if payload.verge_tproxy_port.is_some() || payload.verge_tproxy_enabled.is_some() {
        anyhow::bail!("verge proxy settings are disabled in enterprise managed mode");
    }

    Ok(())
}
