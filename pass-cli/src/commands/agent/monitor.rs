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

use super::find_agent_by_name;
use crate::commands::{OutputFormat, settings_helper};
use crate::helpers::CliPassClient as PassClient;
use crate::utils::format_timestamp_with_time;
use anyhow::{Context, Result, anyhow};
use pass_domain::PersonalAccessTokenId;

pub async fn run(
    client: PassClient,
    name: Option<String>,
    limit: usize,
    output: Option<OutputFormat>,
) -> Result<()> {
    let output = match output {
        Some(fmt) => fmt,
        None => settings_helper::get_default_format(&client)
            .await?
            .unwrap_or(OutputFormat::Human),
    };

    let pat_id = resolve_pat_id(&client, name).await?;

    let records = client
        .list_pat_monitor(&pat_id, limit)
        .await
        .context("Error fetching monitor records")?;

    match output {
        OutputFormat::Json => {
            let json =
                serde_json::to_string_pretty(&records).context("Error serializing records")?;
            println!("{json}");
        }
        OutputFormat::Human => {
            if records.is_empty() {
                println!("No monitor records found");
            } else {
                for entry in records {
                    let time = format_timestamp_with_time(entry.action_time);
                    let action_str = format!("{:?}", entry.action);
                    let payload = &entry.payload;
                    let vault = payload
                        .as_ref()
                        .and_then(|p| p.vault_name.as_deref())
                        .unwrap_or("unknown vault");
                    let item = payload
                        .as_ref()
                        .and_then(|p| p.item_name.as_deref())
                        .unwrap_or("-");
                    let reason = match &payload {
                        Some(p) => {
                            if !p.reason.is_empty() {
                                format!(" reason=\"{}\"", p.reason)
                            } else {
                                "".to_string()
                            }
                        }
                        None => "".to_string(),
                    };
                    println!(
                        "{time} action={action_str} vault=\"{vault}\" item=\"{item}\" {reason}",
                    );
                }
            }
        }
    }

    Ok(())
}

async fn resolve_pat_id(
    client: &PassClient,
    name: Option<String>,
) -> Result<PersonalAccessTokenId> {
    if client.is_user_account() {
        let agent_name =
            name.ok_or_else(|| anyhow!("Agent name is required when logged in as a user account"))?;
        let agent = find_agent_by_name(client, &agent_name).await?;
        Ok(agent.pat_id)
    } else {
        client
            .get_personal_access_token_id()
            .await
            .context("Error retrieving current personal access token ID")
    }
}
