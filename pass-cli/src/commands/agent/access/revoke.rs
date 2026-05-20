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

use crate::helpers::CliPassClient as PassClient;
use anyhow::{Context, Result};
use pass_domain::ShareId;

pub async fn run(client: PassClient, name: String, share_id: String) -> Result<()> {
    let agent = super::super::find_agent_by_name(&client, &name).await?;

    client
        .revoke_personal_access_token_access(&agent.pat_id, &ShareId::new(share_id))
        .await
        .context("Failed to revoke agent access")?;

    println!("Agent access revoked successfully");

    Ok(())
}
