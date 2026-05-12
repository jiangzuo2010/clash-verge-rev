use super::{CmdResult, StringifyErr as _};
use crate::enterprise::{
    EnterpriseAuthCodeRequest, EnterpriseConfigPatch, EnterpriseLoginStart, EnterpriseState, EnterpriseStateView,
    apply_enterprise_runtime_state, complete_enterprise_login as complete_enterprise_login_impl,
    revoke_enterprise_session, shutdown_enterprise_login_callback,
    start_enterprise_login as start_enterprise_login_impl, sync_enterprise_policy_from_server,
};
use clash_verge_logging::{Type, logging};

#[tauri::command]
pub async fn get_enterprise_state() -> CmdResult<EnterpriseStateView> {
    Ok(EnterpriseState::load().await.to_view())
}

#[tauri::command]
pub async fn patch_enterprise_config(patch: EnterpriseConfigPatch) -> CmdResult<EnterpriseStateView> {
    let disabling = patch.enabled == Some(false);
    let mut state = EnterpriseState::load().await;
    if disabling {
        revoke_session_best_effort(&state).await;
    }
    state.config.patch(patch);
    if !state.config.enabled {
        state.clear_session();
        state.clear_policy();
    }
    state.save().await.stringify_err()?;
    apply_enterprise_runtime_state(&state).await.stringify_err()?;
    Ok(state.to_view())
}

#[tauri::command]
pub async fn clear_enterprise_session() -> CmdResult<EnterpriseStateView> {
    shutdown_enterprise_login_callback();
    let mut state = EnterpriseState::load().await;
    revoke_session_best_effort(&state).await;
    state.clear_session();
    state.clear_policy();
    state.save().await.stringify_err()?;
    apply_enterprise_runtime_state(&state).await.stringify_err()?;
    Ok(state.to_view())
}

async fn revoke_session_best_effort(state: &EnterpriseState) {
    if let Err(err) = revoke_enterprise_session(state).await {
        logging!(warn, Type::Cmd, "撤销企业登录会话失败: {}", err);
    }
}

#[tauri::command]
pub async fn start_enterprise_login(open_browser: Option<bool>) -> CmdResult<EnterpriseLoginStart> {
    start_enterprise_login_impl(open_browser.unwrap_or(true))
        .await
        .stringify_err()
}

#[tauri::command]
pub async fn complete_enterprise_login(request: EnterpriseAuthCodeRequest) -> CmdResult<EnterpriseStateView> {
    let state = complete_enterprise_login_impl(request).await.stringify_err()?;
    apply_enterprise_runtime_state(&state).await.stringify_err()?;
    Ok(state.to_view())
}

#[tauri::command]
pub async fn sync_enterprise_policy() -> CmdResult<EnterpriseStateView> {
    let state = sync_enterprise_policy_from_server().await.stringify_err()?;
    apply_enterprise_runtime_state(&state).await.stringify_err()?;
    Ok(state.to_view())
}
