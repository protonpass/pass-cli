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

use super::{AgentOutput, fetch_agent_instructions, find_agent_by_name};
use crate::commands::OutputFormat;
use crate::commands::personal_access_token::{PatExpiration, expiration_to_timestamp};
use crate::helpers::CliPassClient as PassClient;
use anyhow::{Context, Result};

pub async fn run(
    client: PassClient,
    name: String,
    expiration: PatExpiration,
    output: Option<OutputFormat>,
) -> Result<()> {
    let agent = find_agent_by_name(&client, &name).await?;
    let expiration_timestamp = expiration_to_timestamp(&expiration)?;

    let response = client
        .renew_personal_access_token(&agent.pat_id, expiration_timestamp)
        .await
        .context("Failed to renew agent")?;

    let agent_output = AgentOutput {
        token: response.env_var,
        instruction: fetch_agent_instructions().await?,
    };

    match output.unwrap_or(OutputFormat::Json) {
        OutputFormat::Json => {
            let json =
                serde_json::to_string_pretty(&agent_output).context("Error serializing output")?;
            println!("{json}");
        }
        OutputFormat::Human => {
            println!("{}", agent_output.token);
        }
    }

    Ok(())
}
