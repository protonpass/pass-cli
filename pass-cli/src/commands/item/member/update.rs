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

use crate::commands::Role;
use crate::helpers::CliPassClient as PassClient;
use anyhow::{Context, Result};
use pass_domain::{ShareId, ShareRole};

pub async fn run(
    client: PassClient,
    share_id: ShareId,
    member_share_id: ShareId,
    role: Role,
) -> Result<()> {
    let share_role: ShareRole = role.into();

    client
        .update_vault_member(&share_id, &member_share_id, share_role)
        .await
        .context("Error updating item member")?;

    println!("Successfully updated member role");
    Ok(())
}
