use crate::commands::OutputFormat;
use anyhow::Result;
use clap::Subcommand;
use pass::PassClient;

pub mod info;

#[derive(Subcommand)]
pub enum UserCommands {
    #[command(about = "Show user info")]
    Info {
        #[arg(long, default_value = "human")]
        output: OutputFormat,
    },
}

pub async fn run(subcommand: UserCommands, client: PassClient) -> Result<()> {
    match subcommand {
        UserCommands::Info { output } => info::run(client, output).await,
    }
}
