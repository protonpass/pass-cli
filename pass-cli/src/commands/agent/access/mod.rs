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
use anyhow::Result;
use clap::Subcommand;

mod grant;
mod revoke;

#[derive(Subcommand)]
pub enum AgentAccessCommands {
    #[command(about = "Grant vault or item access to an agent")]
    Grant {
        #[arg(help = "Agent name")]
        name: String,
        #[arg(long, help = "Share ID of the vault to grant access to")]
        share_id: Option<String>,
        #[arg(long, help = "Name of the vault to grant access to")]
        vault_name: Option<String>,
        #[arg(long, help = "Specific item ID to grant access to")]
        item_id: Option<String>,
        #[arg(long, help = "Specific item title to grant access to")]
        item_title: Option<String>,
        #[arg(long, default_value = "viewer")]
        role: Role,
    },
    #[command(about = "Revoke vault access from an agent")]
    Revoke {
        #[arg(help = "Agent name")]
        name: String,
        #[arg(long, help = "Share ID to revoke access from")]
        share_id: String,
    },
}

pub async fn run(command: AgentAccessCommands, client: PassClient) -> Result<()> {
    match command {
        AgentAccessCommands::Grant {
            name,
            share_id,
            vault_name,
            item_id,
            item_title,
            role,
        } => {
            grant::run(
                client, name, share_id, vault_name, item_id, item_title, role,
            )
            .await
        }
        AgentAccessCommands::Revoke { name, share_id } => revoke::run(client, name, share_id).await,
    }
}
