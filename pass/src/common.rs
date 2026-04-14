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

use anyhow::{Result, anyhow};

pub(crate) const SUCCESS_CODE: u32 = 1000;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub(crate) struct CodeResponse {
    #[serde(rename = "Code")]
    pub(crate) code: u32,
}

impl CodeResponse {
    pub fn is_success(&self) -> bool {
        self.code == SUCCESS_CODE
    }

    pub fn success_guard(&self) -> Result<()> {
        if !self.is_success() {
            Err(anyhow!("Invalid result code: {}", self.code))
        } else {
            Ok(())
        }
    }
}
