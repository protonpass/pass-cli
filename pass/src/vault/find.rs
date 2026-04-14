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

use crate::{PassClient, PassClientContext};
use anyhow::{Context, Result, anyhow};
use pass_domain::Vault;

impl<C: PassClientContext> PassClient<C> {
    pub async fn find_vault(&self, vault_name: &str) -> Result<Vault> {
        let vaults = self.list_vaults().await.context("Error listing vaults")?;
        let vault = vaults
            .into_iter()
            .find(|v| v.content.name == vault_name)
            .ok_or_else(|| anyhow!("Could not find vault {}", vault_name))?;

        Ok(vault)
    }
}
