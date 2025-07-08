mod generate;
mod score;

use anyhow::Result;
use clap::{Subcommand, ValueEnum};

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
    },
}

pub async fn run(command: &PasswordCommands) -> Result<()> {
    match command {
        PasswordCommands::Generate { command } => generate::run(command).await,
        PasswordCommands::Score { password } => score::score(password),
    }
}
