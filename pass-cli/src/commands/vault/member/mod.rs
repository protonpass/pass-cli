use super::VaultQuery;
use crate::commands::{OutputFormat, Role};
use crate::helpers::CliPassClient as PassClient;
use anyhow::Result;
use clap::Subcommand;
use pass_domain::ShareId;

pub mod list;
pub mod remove;
pub mod update;

#[derive(Subcommand)]
pub enum MemberCommands {
    #[command(about = "List vault members")]
    List {
        #[arg(long, help = "Share ID of the vault")]
        share_id: Option<String>,
        #[arg(long, help = "Name of the vault")]
        vault_name: Option<String>,
        #[arg(long, default_value = "human")]
        output: OutputFormat,
    },
    #[command(about = "Update a vault member's role")]
    Update {
        #[arg(long, help = "Share ID of the vault")]
        share_id: Option<String>,
        #[arg(long, help = "Name of the vault")]
        vault_name: Option<String>,
        #[arg(long, help = "Member share ID")]
        member_share_id: String,
        #[arg(long, help = "New role for the member")]
        role: Role,
    },
    #[command(about = "Remove a vault member")]
    Remove {
        #[arg(long, help = "Share ID of the vault")]
        share_id: Option<String>,
        #[arg(long, help = "Name of the vault")]
        vault_name: Option<String>,
        #[arg(long, help = "Member share ID")]
        member_share_id: String,
    },
}

pub async fn run(client: PassClient, subcommand: MemberCommands) -> Result<()> {
    match subcommand {
        MemberCommands::List {
            share_id,
            vault_name,
            output,
        } => {
            let query = VaultQuery::new(share_id, vault_name)?;
            list::run(client, query, output).await
        }
        MemberCommands::Update {
            share_id,
            vault_name,
            member_share_id,
            role,
        } => {
            let query = VaultQuery::new(share_id, vault_name)?;
            update::run(client, query, ShareId::new(member_share_id), role).await
        }
        MemberCommands::Remove {
            share_id,
            vault_name,
            member_share_id,
        } => {
            let query = VaultQuery::new(share_id, vault_name)?;
            remove::run(client, query, ShareId::new(member_share_id)).await
        }
    }
}
