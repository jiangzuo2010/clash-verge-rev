use super::EnterpriseState;
use crate::core::{CoreManager, handle, proxy_control};
use anyhow::Result;
use clash_verge_logging::{Type, logging};

pub async fn apply_enterprise_runtime_state(state: &EnterpriseState) -> Result<()> {
    if !state.config.enabled {
        let outcome = CoreManager::global().update_config_forced().await?;
        if !outcome.is_valid() {
            anyhow::bail!("{outcome}");
        }
        apply_system_proxy_state().await?;
        handle::Handle::refresh_clash();
        return Ok(());
    }

    if state.is_authenticated() && state.has_valid_cached_policy() {
        let outcome = CoreManager::global().update_config_forced().await?;
        if !outcome.is_valid() {
            fail_closed().await;
            anyhow::bail!("{outcome}");
        }
        if let Err(err) = apply_system_proxy_state().await {
            fail_closed().await;
            anyhow::bail!(err);
        }
        handle::Handle::refresh_clash();
        return Ok(());
    }

    fail_closed().await;
    Ok(())
}

pub async fn ensure_enterprise_runtime_ready_for_core_start() -> Result<()> {
    let state = EnterpriseState::load().await;
    if !state.config.enabled {
        return Ok(());
    }

    if state.is_authenticated() && state.has_valid_cached_policy() {
        return Ok(());
    }

    fail_closed().await;
    anyhow::bail!("enterprise runtime requires login and a valid policy");
}

async fn apply_system_proxy_state() -> Result<()> {
    proxy_control::apply().await?;
    proxy_control::refresh_guard().await?;
    Ok(())
}

async fn fail_closed() {
    if let Err(err) = proxy_control::clear().await {
        logging!(warn, Type::Core, "重置企业代理系统代理失败: {}", err);
    }
    if let Err(err) = CoreManager::global().stop_core().await {
        logging!(warn, Type::Core, "停止企业代理核心失败: {}", err);
    }
    handle::Handle::refresh_clash();
}
