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

pub fn get_linux_os_info() -> OsInfoResult {
    // sysinfo requires instantiating System to access OS info
    let _sys = System::new();

    // Normalize OS name to "Linux" for User-Agent consistency
    let os_name = "Linux".to_string();
    let os_version = System::os_version().unwrap_or_default();
    let platform = Some(System::cpu_arch());

    Ok(OsInfo::new(os_name, os_version, platform))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_linux_os_info() {
        if cfg!(target_os = "linux") {
            let info = get_linux_os_info().expect("should succeed on Linux");
            assert_eq!(info.os_name, "Linux");
            // Version can be empty or contain distro info like "Ubuntu 22.04"
        }
    }
}
