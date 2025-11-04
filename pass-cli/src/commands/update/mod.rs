mod check;
mod download;
mod manifest;
mod platform;
mod replace;
mod state;
mod track;

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

pub use check::check_for_updates_background;
pub use track::get_release_track;

const ENV_NO_UPDATE_CHECK: &str = "PROTON_PASS_NO_UPDATE_CHECK";
const ENV_UPDATE_VERSION_STRATEGY: &str = "PROTON_PASS_UPDATE_VERSION_STRATEGY";
const MANIFEST_BASE_URL: &str = "https://proton.me/download/pass-cli/";

fn get_default_manifest_url() -> String {
    format!("{}versions.json", MANIFEST_BASE_URL)
}

fn get_manifest_url_for_track(track: &str) -> String {
    if track.is_empty() || track == "stable" {
        get_default_manifest_url()
    } else {
        format!("{}versions.{}.json", MANIFEST_BASE_URL, track)
    }
}

#[cfg(debug_assertions)]
const ENV_AUTOUPDATE_URL: &str = "PROTON_PASS_AUTOUPDATE_URL";

// In debug mode, allow changing the auto_update url
#[cfg(debug_assertions)]
async fn get_manifest_url(base_dir: &Path) -> Result<String> {
    if let Ok(url) = std::env::var(ENV_AUTOUPDATE_URL) {
        return Ok(url);
    }

    let track = get_release_track(base_dir).await?;
    Ok(get_manifest_url_for_track(&track))
}

// In release mode, use persistent track with optional env var fallback
#[cfg(not(debug_assertions))]
async fn get_manifest_url(base_dir: &Path) -> Result<String> {
    let track = get_release_track(base_dir).await?;
    Ok(get_manifest_url_for_track(&track))
}

fn is_autoupdate_disabled() -> bool {
    std::env::var(ENV_NO_UPDATE_CHECK).is_ok()
}

fn is_force_update_strategy() -> bool {
    std::env::var(ENV_UPDATE_VERSION_STRATEGY)
        .map(|val| val.to_lowercase() == "force")
        .unwrap_or(false)
}

pub async fn run(yes: bool, set_track: Option<String>, base_dir: PathBuf) -> Result<()> {
    // Handle --set-track flag
    if let Some(track_name) = set_track {
        track::set_persistent_track(&base_dir, &track_name)
            .await
            .context("Failed to set release track")?;
        eprintln!("Update track set to {}", track_name);
        return Ok(());
    }

    if is_autoupdate_disabled() {
        eprintln!("Auto-update is disabled via {}.", ENV_NO_UPDATE_CHECK);
        return Ok(());
    }

    let manifest_url = get_manifest_url(base_dir.as_path()).await?;
    let current_version = env!("CARGO_PKG_VERSION");

    let manifest = match manifest::fetch_manifest(&manifest_url).await {
        Ok(m) => m,
        Err(e) => {
            // Check if it's a 404 error and provide helpful message
            let error_str = e.to_string();
            if error_str.contains("404") {
                eprintln!("Failed to fetch update manifest: {}", e);
                eprintln!("\nThe release track you are on may not exist or may have been removed.");
                eprintln!("Try running:");
                eprintln!("    pass-cli update --set-track stable");
                std::process::exit(1);
            }
            return Err(e).context("Failed to fetch update manifest");
        }
    };

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
