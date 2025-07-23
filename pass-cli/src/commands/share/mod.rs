mod list;

use crate::commands::OutputFormat;
use crate::commands::share::list::ShareListMode;
use anyhow::{Context, Result};
use clap::Subcommand;
use pass::PassClient;

#[derive(Subcommand)]
pub enum ShareCommands {
    #[command(about = "List available shares")]
    List {
        #[arg(long, help = "Only display item shares", default_value = "false")]
        only_items: Option<bool>,
        #[arg(long, help = "Only display vault shares", default_value = "false")]
        only_vaults: Option<bool>,
        #[arg(long, default_value = "human")]
        output: OutputFormat,
    },
}

pub async fn run(command: ShareCommands, client: PassClient) -> Result<()> {
    match command {
        ShareCommands::List {
            only_items,
            only_vaults,
            output,
        } => {
            let mode =
                ShareListMode::from_args(only_vaults.unwrap_or(false), only_items.unwrap_or(false))
                    .context("Error parsing arguments")?;
            list::run(client, mode, output).await
        }
    }
}
