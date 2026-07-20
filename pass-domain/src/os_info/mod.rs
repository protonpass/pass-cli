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

mod linux;
mod macos;
mod windows;

pub use linux::get_linux_os_info;
pub use macos::get_macos_os_info;
pub use windows::get_windows_os_info;

use std::sync::OnceLock;

#[derive(Debug, Clone)]
pub struct OsInfo {
    pub os_name: String,
    pub os_version: String,
    pub platform: Option<String>,
}

impl OsInfo {
    pub fn new(os_name: String, os_version: String, platform: Option<String>) -> Self {
        Self {
            os_name,
            os_version,
            platform,
        }
    }

    pub fn user_agent_os_string(&self) -> String {
        if self.os_version.is_empty() {
            self.os_name.clone()
        } else {
            format!("{} {}", self.os_name, self.os_version)
        }
    }
}

#[derive(Debug, Clone)]
pub struct OsInfoError {
    pub message: String,
}

impl std::fmt::Display for OsInfoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for OsInfoError {}

pub type OsInfoResult = Result<OsInfo, OsInfoError>;

static OS_INFO: OnceLock<OsInfoResult> = OnceLock::new();

pub fn get_os_info() -> OsInfoResult {
    OS_INFO
        .get_or_init(|| {
            #[cfg(target_os = "macos")]
            return get_macos_os_info();
            #[cfg(target_os = "linux")]
            return get_linux_os_info();
            #[cfg(target_os = "windows")]
            return get_windows_os_info();
            #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
            Err(OsInfoError {
                message: "Unsupported OS".into(),
            })
        })
        .clone()
}

pub fn get_user_agent_os_string() -> String {
    get_os_info()
        .map(|info| info.user_agent_os_string())
        .unwrap_or_else(|_| "Unknown".into())
}

pub fn get_platform_string() -> Option<String> {
    get_os_info().ok().and_then(|info| info.platform)
}

pub fn get_formatted_os_for_user_agent() -> String {
    match get_os_info() {
        Ok(info) => {
            let os_part = info.user_agent_os_string();
            if let Some(platform) = info.platform {
                format!("{}; {}", os_part, platform)
            } else {
                os_part
            }
        }
        Err(_) => "Unknown".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_os_info() {
        #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
        {
            let info = get_os_info().expect("should succeed on supported platforms");
            assert!(!info.os_name.is_empty());
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            assert!(get_os_info().is_err());
        }
    }
}
