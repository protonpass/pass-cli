mod login;
mod note;

use anyhow::Result;
use clap::Subcommand;
use pass::PassClient;

#[derive(Subcommand)]
pub enum CreateCommands {
    /// Create a new login item
    Login {
        #[command(flatten)]
        args: login::LoginArgs,
    },
    /// Create a new note item
    Note {
        #[command(flatten)]
        args: note::NoteArgs,
    },
}

pub async fn run(command: CreateCommands, client: PassClient) -> Result<()> {
    match command {
        CreateCommands::Login { args } => login::run(args, client).await,
        CreateCommands::Note { args } => note::run(args, client).await,
    }
}
