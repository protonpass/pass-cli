#[macro_use]
extern crate tracing;

use crate::features::CliClientFeatures;
use anyhow::{Context, Result, anyhow};
use clap::{Parser, Subcommand};
use pass::PassClient;
use std::sync::Arc;

mod client;
mod commands;
mod features;
mod logs;
mod storage;
mod store;
mod utils;

#[derive(Parser)]
#[command(name = "pass")]
#[command(about = "A CLI tool for Proton Pass", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Log in with a given username")]
    Login {
        #[arg(help = "The username to log in with")]
        username: String,
    },

    #[command(about = "Log out of the current session")]
    Logout,
    #[command(about = "Test if the authenticated connection can be established")]
    Test,
    #[command(about = "Show information about the current session")]
    Info,
    #[command(about = "Inject secrets into a file templated with secret references")]
    Inject {
        #[arg(
            long,
            help = "Set filemode for the output file (Unix systems only). It is ignored without the --out-file flag.",
            default_value = "0600"
        )]
        file_mode: String,

        #[arg(short, long, help = "Do not prompt for confirmation")]
        force: bool,

        #[arg(short, long, help = "The filename of a template file to inject")]
        in_file: Option<String>,

        #[arg(
            short,
            long,
            help = "Write the injected template to a file instead of stdout"
        )]
        out_file: Option<String>,
    },
    #[command(about = "Pass secrets as environment variables to an application or script")]
    Run {
        #[arg(
            long = "env-file",
            help = "Enable Dotenv integration with specific Dotenv files to parse",
            action = clap::ArgAction::Append
        )]
        env_files: Vec<String>,

        #[arg(long, help = "Disable masking of secrets on stdout and stderr")]
        no_masking: bool,

        #[arg(
            help = "The command and arguments to execute",
            last = true,
            required = true
        )]
        command: Vec<String>,
    },
    #[command(about = "Vault operations")]
    Vault {
        #[command(subcommand)]
        command: commands::vault::VaultCommands,
    },
    #[command(about = "Item operations")]
    Item {
        #[command(subcommand)]
        command: commands::item::ItemCommands,
    },
    #[command(about = "Invite operations")]
    Invite {
        #[command(subcommand)]
        command: commands::invite::InviteCommands,
    },
    #[command(about = "Password operations")]
    Password {
        #[command(subcommand)]
        command: commands::password::PasswordCommands,
    },
    #[command(about = "Share operations")]
    Share {
        #[command(subcommand)]
        command: commands::share::ShareCommands,
    },
    #[command(about = "User operations")]
    User {
        #[command(subcommand)]
        command: commands::user::UserCommands,
    },
    #[cfg(feature = "internal")]
    #[command(about = "Internal operations")]
    Internal {
        #[command(subcommand)]
        command: commands::internal::InternalCommands,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    logs::setup_logs();
    let cli = Cli::parse();
    let client = client::get_client().await.context("Error getting client")?;
    match &cli.command {
        Commands::Login { username } => return commands::login::run(username, client).await,
        Commands::Password { command } => {
            return commands::password::run(command).await;
        }
        _ => {}
    };

    if !client.is_authenticated().await {
        return Err(anyhow!("This operation requires an authenticated client"));
    }

    let base_dir = utils::get_base_dir().context("Error getting base dir")?;
    let client = PassClient::new(client, Arc::new(CliClientFeatures::new(base_dir)));

    match cli.command {
        Commands::Logout => commands::logout::run(client).await,
        Commands::Test => commands::test::run(client).await,
        Commands::Info => commands::info::run(client).await,
        Commands::Inject {
            file_mode,
            force,
            in_file,
            out_file,
        } => commands::inject::run(file_mode, force, in_file, out_file, client).await,
        Commands::Run {
            env_files,
            no_masking,
            command,
        } => commands::run::run(env_files, no_masking, command, client).await,
        Commands::Vault { command } => commands::vault::run(command, client).await,
        Commands::Item { command } => commands::item::run(command, client).await,
        Commands::Invite { command } => commands::invite::run(command, client).await,
        Commands::Share { command } => commands::share::run(command, client).await,
        Commands::User { command } => commands::user::run(command, client).await,

        #[cfg(feature = "internal")]
        Commands::Internal { command } => commands::internal::run(command, client).await,
        _ => Ok(()),
    }
}
