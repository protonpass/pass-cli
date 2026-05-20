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

mod access;
mod create;
mod delete;
mod list;
mod monitor;
mod renew;

use crate::helpers::CliPassClient as PassClient;
use anyhow::{Context, Result, anyhow};
use clap::Subcommand;

pub const AGENT_INSTRUCTIONS_URL: &str =
    "https://proton.me/download/pass/agent-data/agent-instructions.md";

const MAX_INSTRUCTIONS_SIZE: usize = 128 * 1024;

pub async fn fetch_agent_instructions() -> Result<String> {
    let response = reqwest::get(AGENT_INSTRUCTIONS_URL)
        .await
        .context("Failed to fetch agent instructions")?
        .error_for_status()
        .context("Agent instructions URL returned an error status")?;

    let bytes = response
        .bytes()
        .await
        .context("Failed to read agent instructions response")?;

    if bytes.len() > MAX_INSTRUCTIONS_SIZE {
        anyhow::bail!(
            "Agent instructions response too large ({} bytes, max {MAX_INSTRUCTIONS_SIZE})",
            bytes.len()
        );
    }

    String::from_utf8(bytes.into()).context("Agent instructions response is not valid UTF-8")
}

#[derive(serde::Serialize)]
pub struct AgentOutput {
    pub token: String,
    pub instruction: String,
}

pub async fn find_agent_by_name(
    client: &PassClient,
    name: &str,
) -> Result<pass::PersonalAccessToken> {
    let pats = client
        .list_personal_access_tokens()
        .await
        .context("Failed to list personal access tokens")?;
    pats.into_iter()
        .filter(|p| p.pass_agent)
        .find(|p| p.name == name)
        .ok_or_else(|| anyhow!("Agent not found: {}", name))
}

#[derive(Subcommand)]
pub enum AgentCommands {
    #[command(about = "Create a new agent")]
    Create {
        #[arg(help = "Agent name")]
        name: String,
        #[arg(long, help = "Expiration (1d, 1w, 1m, 3m, 6m, 1y)")]
        expiration: crate::commands::personal_access_token::PatExpiration,
        #[arg(
            long = "vault",
            help = "Vault name to grant access to (can be repeated)"
        )]
        vaults: Vec<String>,
    },
    #[command(about = "List all agents")]
    List {
        #[arg(long)]
        output: Option<crate::commands::OutputFormat>,
    },
    #[command(about = "Delete an agent")]
    Delete {
        #[arg(help = "Agent name")]
        name: String,
    },
    #[command(about = "List monitor audit entries for an agent")]
    Monitor {
        #[arg(help = "Agent name (required when logged in as a user account)")]
        name: Option<String>,
        #[arg(
            long,
            default_value = "100",
            help = "Maximum number of records to show"
        )]
        limit: usize,
        #[arg(long)]
        output: Option<crate::commands::OutputFormat>,
    },
    #[command(about = "Manage agent vault/item access")]
    Access {
        #[command(subcommand)]
        command: access::AgentAccessCommands,
    },
    #[command(about = "Renew an agent token")]
    Renew {
        #[arg(help = "Agent name")]
        name: String,
        #[arg(long, help = "New expiration (1d, 1w, 1m, 3m, 6m, 1y)")]
        expiration: crate::commands::personal_access_token::PatExpiration,
        #[arg(long)]
        output: Option<crate::commands::OutputFormat>,
    },
    #[command(about = "Print agent usage instructions (markdown)")]
    Instructions,
}

pub async fn run(command: AgentCommands, client: PassClient) -> Result<()> {
    match command {
        AgentCommands::Create {
            name,
            expiration,
            vaults,
        } => create::run(client, name, expiration, vaults).await,
        AgentCommands::List { output } => list::run(client, output).await,
        AgentCommands::Delete { name } => delete::run(client, name).await,
        AgentCommands::Monitor {
            name,
            limit,
            output,
        } => monitor::run(client, name, limit, output).await,
        AgentCommands::Access { command } => access::run(command, client).await,
        AgentCommands::Renew {
            name,
            expiration,
            output,
        } => renew::run(client, name, expiration, output).await,
        AgentCommands::Instructions => {
            let instructions = fetch_agent_instructions().await?;
            println!("{}", instructions);
            Ok(())
        }
    }
}
