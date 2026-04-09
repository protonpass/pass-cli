use super::PersonalAccessTokenQuery;
use crate::commands::Role;
use crate::helpers::CliPassClient as PassClient;
use anyhow::Result;
use clap::Subcommand;

mod grant;
mod list_access;
mod revoke;

#[derive(Subcommand)]
pub enum AccessCommands {
    #[command(about = "Grant access to a personal access token")]
    Grant {
        #[arg(long, help = "Personal access token ID", alias = "pat-id")]
        personal_access_token_id: Option<String>,
        #[arg(long, help = "Personal access token name", alias = "pat-name")]
        personal_access_token_name: Option<String>,
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
    #[command(about = "Revoke access from a personal access token")]
    Revoke {
        #[arg(long, help = "Personal access token ID", alias = "pat-id")]
        personal_access_token_id: Option<String>,
        #[arg(long, help = "Personal access token name", alias = "pat-name")]
        personal_access_token_name: Option<String>,
        #[arg(long, help = "Share ID to revoke access from")]
        share_id: String,
    },
    #[command(about = "List access grants for a personal access token")]
    ListAccess {
        #[arg(long, help = "Personal access token ID", alias = "pat-id")]
        personal_access_token_id: Option<String>,
        #[arg(long, help = "Personal access token name", alias = "pat-name")]
        personal_access_token_name: Option<String>,
        #[arg(long)]
        output: Option<crate::commands::OutputFormat>,
    },
}

pub async fn run(command: AccessCommands, client: PassClient) -> Result<()> {
    match command {
        AccessCommands::Grant {
            personal_access_token_id,
            personal_access_token_name,
            share_id,
            vault_name,
            item_id,
            item_title,
            role,
        } => {
            let query = PersonalAccessTokenQuery::new(
                personal_access_token_id,
                personal_access_token_name,
            )?;
            grant::run(
                client, query, share_id, vault_name, item_id, item_title, role,
            )
            .await
        }
        AccessCommands::Revoke {
            personal_access_token_id,
            personal_access_token_name,
            share_id,
        } => {
            let query = PersonalAccessTokenQuery::new(
                personal_access_token_id,
                personal_access_token_name,
            )?;
            revoke::run(client, query, share_id).await
        }
        AccessCommands::ListAccess {
            personal_access_token_id,
            personal_access_token_name,
            output,
        } => {
            let query = PersonalAccessTokenQuery::new(
                personal_access_token_id,
                personal_access_token_name,
            )?;
            list_access::run(client, query, output).await
        }
    }
}
