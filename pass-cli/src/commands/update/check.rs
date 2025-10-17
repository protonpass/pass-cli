use anyhow::Result;
use std::path::Path;

use super::{get_manifest_url, is_autoupdate_disabled, state};
use crate::commands::update::{manifest, platform};

pub async fn check_for_updates_background(base_dir: &Path) -> Result<()> {
    if is_autoupdate_disabled() {
        return Ok(());
    }

    if !atty::is(atty::Stream::Stderr) {
        return Ok(());
    }

    if !state::should_check_for_updates(base_dir).await? {
        return Ok(());
    }

    let manifest_url = get_manifest_url();
    let manifest = match manifest::fetch_manifest(&manifest_url).await {
        Ok(m) => m,
        Err(_) => {
            let _ = state::update_last_check(base_dir).await;
            return Ok(());
        }
    };

    if manifest.format_version != 1 {
        let _ = state::update_last_check(base_dir).await;
        return Ok(());
    }

    let version_info = &manifest.pass_cli_versions;
    let latest_version = &version_info.version;
    let current_version = env!("CARGO_PKG_VERSION");

    let is_newer = match platform::is_newer_version(latest_version, current_version) {
        Ok(newer) => newer,
        Err(_) => {
            let _ = state::update_last_check(base_dir).await;
            return Ok(());
        }
    };

    if is_newer {
        eprintln!(
            "\nNew update available: v{current_version} -> v{latest_version} (run \"protonpass update\")\n",
        );
    }

    let _ = state::update_last_check(base_dir).await;

    Ok(())
}
