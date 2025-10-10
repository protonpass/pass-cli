mod check;
mod download;
mod manifest;
mod platform;
mod replace;
mod state;

use anyhow::{Context, Result};
use std::path::PathBuf;

pub use check::check_for_updates_background;

const ENV_NO_UPDATE_CHECK: &str = "PROTON_PASS_NO_UPDATE_CHECK";
const ENV_UPDATE_VERSION_STRATEGY: &str = "PASS_CLI_UPDATE_VERSION_STRATEGY";
const DEFAULT_MANIFEST_URL: &str = "https://protonapps.com/download/pass-cli/versions.json";

#[cfg(debug_assertions)]
const ENV_AUTOUPDATE_URL: &str = "PROTON_PASS_AUTOUPDATE_URL";

// In debug mode, allow changing the auto_update url
#[cfg(debug_assertions)]
fn get_manifest_url() -> String {
    std::env::var(ENV_AUTOUPDATE_URL).unwrap_or_else(|_| DEFAULT_MANIFEST_URL.to_string())
}

// In release mode, enforce the manifest url
#[cfg(not(debug_assertions))]
fn get_manifest_url() -> String {
    DEFAULT_MANIFEST_URL.to_string()
}

fn is_autoupdate_disabled() -> bool {
    std::env::var(ENV_NO_UPDATE_CHECK).is_ok()
}

fn is_force_update_strategy() -> bool {
    std::env::var(ENV_UPDATE_VERSION_STRATEGY)
        .map(|val| val.to_lowercase() == "force")
        .unwrap_or(false)
}

pub async fn run(yes: bool, base_dir: PathBuf) -> Result<()> {
    if is_autoupdate_disabled() {
        eprintln!("Auto-update is disabled via {}.", ENV_NO_UPDATE_CHECK);
        return Ok(());
    }

    let manifest_url = get_manifest_url();
    let current_version = env!("CARGO_PKG_VERSION");

    let manifest = manifest::fetch_manifest(&manifest_url)
        .await
        .context("Failed to fetch update manifest")?;

    let version_info = &manifest.pass_cli_versions;
    let latest_version = &version_info.version;

    let force_update = is_force_update_strategy();
    if force_update {
        println!("Force update strategy enabled, skipping version check.");
    } else if !platform::is_newer_version(latest_version, current_version)? {
        println!("Already up to date (v{}).", current_version);
        return Ok(());
    }

    let platform_info = platform::get_platform_info()?;
    let binary_info = version_info
        .urls
        .get(&platform_info.os)
        .and_then(|arch_map| arch_map.get(&platform_info.arch))
        .context(format!(
            "No update available for this platform ({}/{}).",
            platform_info.os, platform_info.arch
        ))?;

    if !yes {
        println!("Update pass-cli v{current_version} → v{latest_version}? [Y/n]");
        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .context("Failed to read user input")?;
        let input = input.trim().to_lowercase();

        if input != "y" && input != "Y" && !input.is_empty() {
            println!("No changes made.");
            return Ok(());
        }
    }

    println!("Downloading pass-cli v{}...", latest_version);
    let temp_file = download::download_binary(&binary_info.url, &binary_info.hash)
        .await
        .context("Failed to download binary")?;

    println!("Installing...");
    replace::replace_binary(&temp_file)
        .await
        .context("Failed to replace binary")?;

    println!("Updated to v{}.", latest_version);

    state::update_last_check(&base_dir).await?;

    Ok(())
}
