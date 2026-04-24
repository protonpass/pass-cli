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

#[derive(serde::Serialize)]
struct PersonalAccessTokenView<'a> {
    #[serde(flatten)]
    pat: &'a pass::PersonalAccessToken,
    is_agent: bool,
}

pub async fn run(client: PassClient, output: Option<OutputFormat>) -> Result<()> {
    let output = match output {
        Some(fmt) => fmt,
        None => settings_helper::get_default_format(&client)
            .await?
            .unwrap_or(OutputFormat::Human),
    };

    let personal_access_tokens = client
        .list_personal_access_tokens()
        .await
        .context("Failed to list personal access tokens")?;

    match output {
        OutputFormat::Json => {
            let views: Vec<PersonalAccessTokenView> = personal_access_tokens
                .iter()
                .map(|pat| PersonalAccessTokenView {
                    is_agent: pat.pass_agent,
                    pat,
                })
                .collect();
            let json = serde_json::to_string_pretty(&views)
                .context("Error serializing personal access tokens")?;
            println!("{json}");
        }
        OutputFormat::Human => {
            if personal_access_tokens.is_empty() {
                println!("No personal access tokens found");
            } else {
                for pat in &personal_access_tokens {
                    let agent_prefix = if pat.pass_agent { "[Agent] " } else { "" };
                    let expiration = match pat.expire_time {
                        Some(ts) => format!(" (expires: {})", format_date(ts)),
                        None => String::new(),
                    };
                    println!(
                        "- [{}]: {}{}{}",
                        pat.pat_id, agent_prefix, pat.name, expiration
                    );
                }
            }
        }
    }

    Ok(())
}
