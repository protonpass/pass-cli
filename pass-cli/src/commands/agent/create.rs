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

use super::{AgentOutput, fetch_agent_instructions};
use crate::commands::personal_access_token::{PatExpiration, expiration_to_timestamp};
use crate::helpers::CliPassClient as PassClient;
use anyhow::{Context, Result};
use pass::CreatePersonalAccessTokenArgs;
use pass_domain::ShareRole;

pub async fn run(
    client: PassClient,
    name: String,
    expiration: PatExpiration,
    vaults: Vec<String>,
) -> Result<()> {
    let expiration_timestamp = expiration_to_timestamp(&expiration)?;

    let args =
        CreatePersonalAccessTokenArgs::new(name, expiration_timestamp)?.with_pass_agent_flag();

    let response = client
        .create_personal_access_token(args)
        .await
        .context("Failed to create agent")?;

    let pat_id = response.personal_access_token_id.clone();

    // Grant access to each requested vault
    for vault_name in vaults {
        let share_id = client
            .find_vault(&vault_name)
            .await
            .with_context(|| format!("Failed to find vault: {}", vault_name))?
            .share_id;

        client
            .grant_personal_access_token_access(&pat_id, &share_id, None, &ShareRole::Viewer)
            .await
            .with_context(|| format!("Failed to grant access to vault: {}", vault_name))?;
    }

    let output = AgentOutput {
        token: response.env_var,
        instruction: fetch_agent_instructions().await?,
    };
    let json = serde_json::to_string_pretty(&output).context("Error serializing output")?;
    println!("{json}");

    Ok(())
}
