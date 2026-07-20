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

pub fn get_windows_os_info() -> OsInfoResult {
    let os_name = "Windows".to_string();
    let os_version = System::kernel_version().unwrap_or_else(|| "10.0.0".to_string());
    let platform = Some(System::cpu_arch());

    Ok(OsInfo::new(os_name, os_version, platform))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_windows_os_info() {
        let info = get_windows_os_info().unwrap();
        assert_eq!(info.os_name, "Windows");
        assert!(!info.os_version.is_empty());
    }
}
