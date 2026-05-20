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

use crate::commands::{OutputFormat, settings_helper};
use crate::helpers::CliPassClient as PassClient;
use crate::utils::format_date;
use anyhow::{Context, Result};

pub async fn run(client: PassClient, output: Option<OutputFormat>) -> Result<()> {
    let output = match output {
        Some(fmt) => fmt,
        None => settings_helper::get_default_format(&client)
            .await?
            .unwrap_or(OutputFormat::Human),
    };

    let all_pats = client
        .list_personal_access_tokens()
        .await
        .context("Failed to list personal access tokens")?;

    let agents: Vec<_> = all_pats.into_iter().filter(|p| p.pass_agent).collect();

    match output {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&agents).context("Error serializing agents")?;
            println!("{json}");
        }
        OutputFormat::Human => {
            if agents.is_empty() {
                println!("No agents found");
            } else {
                for agent in agents {
                    let expiration = match agent.expire_time {
                        Some(ts) => format!(" (expires: {})", format_date(ts)),
                        None => String::new(),
                    };
                    println!("- [{}]: {}{}", agent.pat_id, agent.name, expiration);
                }
            }
        }
    }

    Ok(())
}
