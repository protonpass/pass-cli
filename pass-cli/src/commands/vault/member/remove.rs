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

use super::super::VaultQuery;
use crate::helpers::CliPassClient as PassClient;
use anyhow::{Context, Result};
use pass_domain::ShareId;

pub async fn run(client: PassClient, query: VaultQuery, member_share_id: ShareId) -> Result<()> {
    let share_id = query.resolve(&client).await?;
    client
        .remove_vault_member(&share_id, &member_share_id)
        .await
        .context("Error removing vault member")?;

    println!("Successfully removed member from vault");
    Ok(())
}
