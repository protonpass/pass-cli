mod credit_card;
mod custom;
mod identity;
mod login;
mod note;
mod ssh_key;
mod wifi;

use crate::helpers::CliPassClient as PassClient;
use anyhow::Result;
use clap::Subcommand;

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
    /// Create a new credit card item
    #[command(name = "credit-card")]
    CreditCard {
        #[command(flatten)]
        args: credit_card::CreditCardArgs,
    },
    /// Create a new WiFi item
    Wifi {
        #[command(flatten)]
        args: wifi::WifiArgs,
    },
    /// Create a new custom item
    Custom {
        #[command(flatten)]
        args: custom::CustomArgs,
    },
    /// Create a new identity item
    Identity {
        #[command(flatten)]
        args: identity::IdentityArgs,
    },
    /// Create a new SSH key item
    #[command(name = "ssh-key")]
    SshKey {
        #[command(flatten)]
        args: ssh_key::SshKeyArgs,
    },
}

pub async fn run(command: CreateCommands, client: PassClient) -> Result<()> {
    match command {
        CreateCommands::Login { args } => login::run(args, client).await,
        CreateCommands::Note { args } => note::run(args, client).await,
        CreateCommands::CreditCard { args } => credit_card::run(args, client).await,
        CreateCommands::Wifi { args } => wifi::run(args, client).await,
        CreateCommands::Custom { args } => custom::run(args, client).await,
        CreateCommands::Identity { args } => identity::run(args, client).await,
        CreateCommands::SshKey { args } => ssh_key::run(args, client).await,
    }
}
