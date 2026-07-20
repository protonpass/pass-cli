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

use super::{OsInfo, OsInfoResult};
use sysinfo::System;

pub fn get_macos_os_info() -> OsInfoResult {
    // sysinfo requires instantiating System to access OS info
    let _sys = System::new();

    let version_str = System::os_version().unwrap_or_else(|| "0.0.0".to_string());
    let os_version = parse_macos_version(&version_str);
    let platform = Some(System::cpu_arch());

    Ok(OsInfo::new("Mac OS X".into(), os_version, platform))
}

fn parse_macos_version(version: &str) -> String {
    let version = version.trim();
    if version.is_empty() {
        return "0.0.0".into();
    }
    let parts: Vec<&str> = version.split('.').collect();
    match parts.len() {
        0 => "0.0.0".into(),
        1 => format!("{}.0.0", parts[0]),
        2 => format!("{}.{}.0", parts[0], parts[1]),
        _ => format!(
            "{}.{}.{}",
            parts.first().copied().unwrap_or("0"),
            parts.get(1).copied().unwrap_or("0"),
            parts.get(2).copied().unwrap_or("0")
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_macos_version_full() {
        assert_eq!(parse_macos_version("15.1.0"), "15.1.0");
    }
    #[test]
    fn test_parse_macos_version_minor() {
        assert_eq!(parse_macos_version("15.1"), "15.1.0");
    }
    #[test]
    fn test_parse_macos_version_major_only() {
        assert_eq!(parse_macos_version("15"), "15.0.0");
    }
    #[test]
    fn test_parse_macos_version_whitespace() {
        assert_eq!(parse_macos_version(" 15.1.0 "), "15.1.0");
    }
    #[test]
    fn test_parse_macos_version_empty() {
        assert_eq!(parse_macos_version(""), "0.0.0");
    }

    #[test]
    fn test_get_macos_os_info() {
        if cfg!(target_os = "macos") {
            let info = get_macos_os_info().expect("should succeed on macOS");
            assert_eq!(info.os_name, "Mac OS X");
            assert!(!info.os_version.is_empty());
        }
    }
}
