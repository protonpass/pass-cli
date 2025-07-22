mod generate;
mod score;

use anyhow::Result;
use clap::{Subcommand, ValueEnum};
use crate::commands::OutputFormat;

#[derive(ValueEnum, Clone, Debug)]
pub enum PasswordFormat {
    Random,
    Memorable,
}

#[derive(Subcommand)]
pub enum PasswordCommands {
    #[command(about = "Generate a password")]
    Generate {
        #[command(subcommand)]
        command: generate::GeneratePasswordCommand,
    },
    #[command(about = "Score a password")]
    Score {
        #[arg(help = "Password to score")]
        password: String,
        #[arg(long, default_value = "human")]
        output: OutputFormat,
    },
}

pub async fn run(command: &PasswordCommands) -> Result<()> {
    match command {
        PasswordCommands::Generate { command } => generate::run(command).await,
        PasswordCommands::Score { password , output} => score::score(password, output),
    }
}
