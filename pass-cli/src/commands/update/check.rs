/*
 *  Copyright (c) 2026 Proton AG
 *  This file is part of Proton AG and Proton Pass.
 *
 *  Proton Pass is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  Proton Pass is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with Proton Pass.  If not, see <https://www.gnu.org/licenses/>.
 *
 */

use anyhow::Result;
use std::io::IsTerminal;
use std::path::Path;

use super::{get_manifest_url, is_autoupdate_disabled, state};
use crate::commands::update::{manifest, platform};

pub async fn check_for_updates_background(base_dir: &Path) -> Result<()> {
    if is_autoupdate_disabled() {
        return Ok(());
    }

    if !std::io::stderr().is_terminal() {
        return Ok(());
    }

    if !state::should_check_for_updates(base_dir).await? {
        return Ok(());
    }

    let manifest_url = match get_manifest_url(base_dir).await {
        Ok(url) => url,
        Err(_) => {
            let _ = state::update_last_check(base_dir).await;
            return Ok(());
        }
    };

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
            "\nNew update available: v{current_version} -> v{latest_version} (run \"pass-cli update\")\n",
        );
    }

    let _ = state::update_last_check(base_dir).await;

    Ok(())
}
