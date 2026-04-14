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

use crate::commands::{OutputFormat, Role};
use crate::helpers::CliPassClient as PassClient;
use anyhow::{Context, Result, anyhow};
use clap::Subcommand;
use pass_domain::ShareId;

pub enum VaultQuery {
    ShareId(ShareId),
    VaultName(String),
}

impl VaultQuery {
    pub fn new(share_id: Option<String>, name: Option<String>) -> Result<Self> {
        match (share_id, name) {
            (Some(share_id), None) => Ok(Self::ShareId(ShareId::new(share_id))),
            (None, Some(vault_name)) => Ok(Self::VaultName(vault_name)),

            _ => Err(anyhow!("Please provide either share-id or vault name")),
        }
    }

    pub async fn resolve(&self, client: &PassClient) -> Result<ShareId> {
        match self {
            VaultQuery::ShareId(id) => Ok(id.clone()),
            VaultQuery::VaultName(vault) => {
                let vault = client
                    .find_vault(vault)
                    .await
                    .context("Error finding vault")?;
                Ok(vault.share_id)
            }
        }
    }
}

pub mod create;
pub mod delete;
pub mod list;
pub mod member;
pub mod share;
mod transfer;
mod update;

#[derive(Subcommand)]
pub enum VaultCommands {
    #[command(about = "List vaults")]
    List {
        #[arg(long)]
        output: Option<OutputFormat>,
    },
    #[command(about = "Create a new vault")]
    Create {
        #[arg(long, help = "Name of the vault")]
        name: String,
    },
    #[command(about = "Update a vault")]
    Update {
        #[arg(long, help = "Share ID of the vault")]
        share_id: Option<String>,
        #[arg(long, help = "Name of the vault")]
        vault_name: Option<String>,
        #[arg(long, help = "New name of the vault")]
        name: String,
    },
    #[command(about = "Manage vault members", subcommand)]
    Member(member::MemberCommands),
    #[command(about = "Delete a vault")]
    Delete {
        #[arg(long, help = "Share ID of the vault to delete")]
        share_id: Option<String>,
        #[arg(long, help = "Name of the vault to delete")]
        vault_name: Option<String>,
    },
    #[command(about = "Share a vault with someone")]
    Share {
        #[arg(long, help = "Share ID of the vault to share")]
        share_id: Option<String>,
        #[arg(long, help = "Name of the vault to share")]
        vault_name: Option<String>,
        #[arg(help = "Email address to share with")]
        email: String,
        #[arg(long, default_value = "viewer")]
        role: Role,
    },
    #[command(about = "Transfer the ownership of one of your vaults")]
    Transfer {
        #[arg(long, help = "Share ID of the vault to transfer ownership")]
        share_id: Option<String>,
        #[arg(long, help = "Name of the vault to to transfer ownership")]
        vault_name: Option<String>,
        #[arg(help = "Member Share ID of the new owner of the vault")]
        member_share_id: String,
    },
}

pub async fn run(subcommand: VaultCommands, client: PassClient) -> Result<()> {
    match subcommand {
        VaultCommands::List { output } => list::run(client, output).await,
        VaultCommands::Update {
            share_id,
            vault_name,
            name,
        } => {
            let query = VaultQuery::new(share_id, vault_name)?;
            update::run(client, query, name).await
        }
        VaultCommands::Create { name } => create::run(client, name).await,
        VaultCommands::Member(member_cmd) => member::run(client, member_cmd).await,
        VaultCommands::Delete {
            share_id,
            vault_name,
        } => {
            let query = VaultQuery::new(share_id, vault_name)?;
            delete::run(client, query).await
        }
        VaultCommands::Share {
            share_id,
            vault_name,
            email,
            role,
        } => {
            let query = VaultQuery::new(share_id, vault_name)?;
            share::run(client, query, email, role).await
        }
        VaultCommands::Transfer {
            share_id,
            vault_name,
            member_share_id,
        } => {
            let query = VaultQuery::new(share_id, vault_name)?;
            transfer::run(client, query, ShareId::new(member_share_id)).await
        }
    }
}
