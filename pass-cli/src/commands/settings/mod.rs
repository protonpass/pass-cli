use crate::helpers::CliPassClient as PassClient;
use anyhow::Result;
use clap::Subcommand;

pub mod set;
pub mod unset;
pub mod view;

#[derive(Subcommand)]
pub enum SettingsCommands {
    #[command(about = "View all current settings")]
    View,

    #[command(about = "Set a setting value", subcommand)]
    Set(set::SetCommands),

    #[command(about = "Unset (clear) a setting", subcommand)]
    Unset(unset::UnsetCommands),
}

pub async fn run(subcommand: SettingsCommands, client: PassClient) -> Result<()> {
    match subcommand {
        SettingsCommands::View => view::run(client).await,
        SettingsCommands::Set(cmd) => set::run(cmd, client).await,
        SettingsCommands::Unset(cmd) => unset::run(cmd, client).await,
    }
}
