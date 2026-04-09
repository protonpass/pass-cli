mod create;

use crate::commands::OutputFormat;
use crate::commands::item::common::ShareQuery;
use crate::commands::settings_helper;
use crate::helpers::CliPassClient as PassClient;
use anyhow::Result;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum AliasCommands {
    #[command(about = "Create a new alias")]
    Create {
        #[arg(long, help = "Share ID of the vault where the alias will be created")]
        share_id: Option<String>,
        #[arg(long, help = "Name of the vault where the alias will be created")]
        vault_name: Option<String>,
        #[arg(
            long,
            help = "Prefix of the alias. The resulting email will be [prefix].[suffix]"
        )]
        prefix: String,
        #[arg(long, help = "Output format", default_value = "human")]
        output: OutputFormat,
    },
}

pub async fn run(subcommand: AliasCommands, client: PassClient) -> Result<()> {
    match subcommand {
        AliasCommands::Create {
            mut share_id,
            vault_name,
            prefix,
            output,
        } => {
            // Apply default vault if both are None
            if share_id.is_none() && vault_name.is_none() {
                share_id = settings_helper::get_default_share_id(&client)
                    .await?
                    .map(|id| id.to_string());
            }

            let share_query = ShareQuery::new(share_id, vault_name)?;
            create::run(client, share_query, prefix, output).await
        }
    }
}
