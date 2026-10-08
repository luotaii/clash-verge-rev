use crate::config::{Config, IVerge};
use anyhow::{Result, bail};
use clash_verge_draft::SharedDraft;
use serde_yaml_ng::{Mapping, Value};
use tokio::sync::MutexGuard;

pub async fn patch_clash(patch: &Mapping) -> Result<()> {
    let config_write = Config::try_lock_config_write()?;
    super::executor::apply(
        &config_write,
        super::executor::Patch::Clash(patch),
        super::effects::clash_effects(patch),
    )
    .await
}

fn tun_settings_patch(mut tun: Mapping, interface_name: Option<&str>) -> Result<(Mapping, IVerge)> {
    if let Some(name) = interface_name {
        if name.trim().is_empty() {
            bail!("a manual outbound interface must have a name");
        }
        tun.insert("auto-detect-interface".into(), false.into());
    }
    let mut clash = Mapping::new();
    clash.insert("tun".into(), Value::Mapping(tun));
    clash.insert("interface-name".into(), interface_name.unwrap_or_default().into());
    let verge = IVerge {
        enable_tun_compatibility_guard: Some(cfg!(target_os = "windows") && interface_name.is_some()),
        ..IVerge::default()
    };
    Ok((clash, verge))
}

pub async fn patch_tun_settings(tun: Mapping, interface_name: Option<&str>) -> Result<()> {
    let (clash, verge) = tun_settings_patch(tun, interface_name)?;
    let config_write = Config::try_lock_config_write()?;
    let mut effects = super::effects::clash_effects(&clash);
    effects.extend(super::effects::verge_effects(&verge));
    super::executor::apply(
        &config_write,
        super::executor::Patch::Tun {
            clash: &clash,
            verge: &verge,
        },
        effects,
    )
    .await
}

/// Apply a patch, then reconcile TUN when its setting changes.
///
/// TUN patches do not always produce a Run State transition, so reconciliation is explicit.
pub async fn patch_verge(patch: &IVerge, not_save_file: bool) -> Result<()> {
    apply_verge_patch(patch, not_save_file).await?;
    if patch.enable_tun_mode.is_some() {
        super::reconcile_tun_availability().await;
    }
    Ok(())
}

/// Apply a patch without post-update reconciliation.
pub(super) async fn apply_verge_patch(patch: &IVerge, not_save_file: bool) -> Result<()> {
    let config_write = Config::try_lock_config_write()?;
    apply_verge_patch_locked(&config_write, patch, not_save_file).await
}

/// Apply a patch with the shared configuration write lock already held.
/// Callers must pass the guard returned by [`Config::lock_config_write`].
pub(super) async fn apply_verge_patch_locked(
    _config_write: &MutexGuard<'_, ()>,
    patch: &IVerge,
    not_save_file: bool,
) -> Result<()> {
    super::executor::apply(
        _config_write,
        super::executor::Patch::Verge {
            patch,
            persist: !not_save_file,
        },
        super::effects::verge_effects(patch),
    )
    .await
}

pub async fn fetch_verge_config() -> Result<SharedDraft<IVerge>> {
    let draft = Config::verge().await;
    let data = draft.data_arc();
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_interface_and_guard_are_one_settings_patch() -> Result<()> {
        let mut tun = Mapping::new();
        tun.insert("auto-detect-interface".into(), true.into());
        let (clash, verge) = tun_settings_patch(tun, Some("WLAN"))?;
        assert_eq!(clash.get("interface-name").and_then(Value::as_str), Some("WLAN"));
        assert_eq!(clash["tun"]["auto-detect-interface"].as_bool(), Some(false));
        assert_eq!(verge.enable_tun_compatibility_guard, Some(cfg!(target_os = "windows")));
        let (clash, verge) = tun_settings_patch(Mapping::new(), None)?;
        assert_eq!(clash.get("interface-name").and_then(Value::as_str), Some(""));
        assert_eq!(verge.enable_tun_compatibility_guard, Some(false));
        assert!(tun_settings_patch(Mapping::new(), Some(" ")).is_err());
        Ok(())
    }
}
