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

use anyhow::{Context, Result};

pub struct PlatformInfo {
    pub os: String,
    pub arch: String,
}

pub fn get_platform_info() -> Result<PlatformInfo> {
    let os = match std::env::consts::OS {
        "windows" => "windows",
        "macos" => "macos",
        "linux" => "linux",
        other => return Err(anyhow::anyhow!("Unsupported OS: {other}")),
    };

    let arch = match std::env::consts::ARCH {
        "x86_64" => "x86_64",
        "aarch64" => "aarch64",
        other => return Err(anyhow::anyhow!("Unsupported architecture: {other}")),
    };

    Ok(PlatformInfo {
        os: os.to_string(),
        arch: arch.to_string(),
    })
}

pub fn is_newer_version(latest: &str, current: &str) -> Result<bool> {
    let latest_ver = semver::Version::parse(latest)
        .with_context(|| format!("Invalid version in manifest: {latest}"))?;
    let current_ver = semver::Version::parse(current)
        .with_context(|| format!("Invalid current version: {current}"))?;

    Ok(latest_ver > current_ver)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_comparison() {
        assert!(is_newer_version("1.2.3", "1.2.2").unwrap());
        assert!(is_newer_version("2.0.0", "1.9.9").unwrap());
        assert!(is_newer_version("1.3.0", "1.2.9").unwrap());

        assert!(!is_newer_version("1.2.3", "1.2.3").unwrap());
        assert!(!is_newer_version("1.2.2", "1.2.3").unwrap());
        assert!(!is_newer_version("1.9.9", "2.0.0").unwrap());
    }

    #[test]
    fn test_platform_detection() {
        let info = get_platform_info().unwrap();

        // Verify we get valid platform info
        assert!(!info.os.is_empty());
        assert!(!info.arch.is_empty());

        // Should be one of the supported platforms
        assert!(matches!(info.os.as_str(), "windows" | "macos" | "linux"));
        assert!(matches!(info.arch.as_str(), "x86_64" | "aarch64"));
    }
}
