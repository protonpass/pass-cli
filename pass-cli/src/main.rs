/*
 *  Copyright (c) 2026 Proton AG
 *  This file is part of Proton AG and Proton Pass.
 *
 *  Proton Pass is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  Proton Pass is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with Proton Pass.  If not, see <https://www.gnu.org/licenses/>.
 *
 */

#[macro_use]
extern crate tracing;

use crate::auth::cli_credential_provider::PERSONAL_ACCESS_TOKEN_ENV_VAR;
use crate::features::CliClientFeatures;
use crate::helpers::CliPassClient as PassClient;
use anyhow::{Context, Result, anyhow};
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{Shell, generate};
use pass::AnyhowErrorExt;
use std::sync::Arc;
use zeroizing_alloc::ZeroAlloc;

mod auth;
mod client;
mod commands;
mod constants;
mod features;
mod helpers;
mod logs;
mod storage;
mod telemetry;
mod utils;

#[global_allocator]
static ALLOC: ZeroAlloc<std::alloc::System> = ZeroAlloc(std::alloc::System);

const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), " (", env!("GIT_HASH"), ")");

#[derive(Parser)]
#[command(name = "Proton Pass CLI")]
#[command(about = "A CLI tool for Proton Pass", long_about = None)]
#[command(version = VERSION)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Manage AI agents")]
    Agent {
        #[command(subcommand)]
        command: commands::agent::AgentCommands,
    },
    #[command(about = "Log in (defaults to web login)")]
    Login {
        #[arg(help = "The username to log in with (for interactive mode)")]
        username: Option<String>,

        #[arg(long, help = "Use interactive login mode")]
        interactive: bool,

        #[arg(
            long,
            help = "Personal access token (format: pst_<token>::<key>)",
            alias = "personal-access-token"
        )]
        pat: Option<String>,
    },

    #[command(about = "Log out of the current session")]
    Logout {
        #[arg(long, help = "Force logout even if remote logout fails")]
        force: bool,
    },
    #[command(about = "Test if the authenticated connection can be established")]
    Test,
    #[command(about = "Show information about the current session")]
    Info {
        #[arg(short, long, value_enum, help = "Output format")]
        output: Option<commands::OutputFormat>,
    },
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
    #[command(about = "Personal Access Token operations", alias = "pat")]
    PersonalAccessToken {
        #[command(subcommand)]
        command: commands::personal_access_token::PersonalAccessTokenCommands,
    },
    #[command(about = "TOTP operations")]
    Totp {
        #[command(subcommand)]
        command: commands::totp::TotpCommands,
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
    #[command(about = "Session operations")]
    Session {
        #[command(subcommand)]
        command: commands::session::SessionCommands,
    },
    #[command(about = "SSH agent operations")]
    SshAgent {
        #[command(subcommand)]
        command: commands::ssh_agent::SshAgentCommands,
    },
    #[command(about = "Manage persistent settings")]
    Settings {
        #[command(subcommand)]
        command: commands::settings::SettingsCommands,
    },
    #[cfg(feature = "internal")]
    #[command(about = "Internal operations")]
    Internal {
        #[command(subcommand)]
        command: commands::internal::InternalCommands,
    },
    #[command(about = "Check for and install updates")]
    Update {
        #[arg(short, long, help = "Skip confirmation prompt")]
        yes: bool,
        #[arg(
            long,
            help = "Change the release track to check updates (default: stable)"
        )]
        set_track: Option<String>,
    },
    #[command(about = "Reach to us if you need help")]
    Support,
    #[command(about = "Generate shell completion scripts", hide = true)]
    Completions {
        #[arg(value_enum, help = "Shell to generate completions for")]
        shell: Shell,
    },
}

impl Commands {
    pub fn is_force_logout(&self) -> bool {
        if let Commands::Logout { force } = self {
            *force
        } else {
            false
        }
    }

    pub fn is_logout(&self) -> bool {
        matches!(self, Commands::Logout { .. })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    if let Err(e) = run().await {
        if e.is_session_invalidated() {
            eprintln!(
                "Your session has been invalidated and you have been logged out automatically."
            );
            eprintln!("Please log in again with: pass login");
            std::process::exit(1);
        }
        return Err(e);
    }

    Ok(())
}

async fn run() -> Result<()> {
    logs::setup_logs();
    let cli = Cli::parse();

    if cli.command.is_force_logout() {
        return commands::logout::force_logout().await;
    }

    if let Commands::Completions { shell } = cli.command {
        let mut cmd = Cli::command();
        generate(shell, &mut cmd, "pass-cli", &mut std::io::stdout());
        return Ok(());
    }

    let base_dir = utils::get_base_dir().context("Error getting base dir")?;

    // Check for updates in the background (non-blocking, weekly check)
    // This runs for all commands except update itself to avoid recursion
    if !matches!(
        cli.command,
        Commands::Update { .. } | Commands::Completions { .. }
    ) {
        let _ = commands::update::check_for_updates_background(&base_dir).await;
    }

    let client_features = CliClientFeatures::new(base_dir.clone())
        .await
        .context("Error creating client features")?;
    let client_features = Arc::new(client_features);

    let (client, store) = client::get_client(base_dir.clone(), client_features.clone())
        .await
        .context("Error getting client")?;
    match &cli.command {
        Commands::Login {
            username,
            interactive,
            ..
        } => {
            // Extract personal_access_token field when feature is enabled
            if let Commands::Login {
                pat: personal_access_token,
                ..
            } = &cli.command
            {
                // Route to personal access token login if --service-account is provided
                if personal_access_token.is_some()
                    || std::env::var(PERSONAL_ACCESS_TOKEN_ENV_VAR).is_ok()
                {
                    return commands::login_pat::run(
                        personal_access_token.clone(),
                        client,
                        client_features,
                        store,
                    )
                    .await;
                }
            }

            return commands::login::run(
                username.as_deref(),
                *interactive,
                client,
                client_features,
                store,
            )
            .await;
        }
        Commands::Password { command } => {
            return commands::password::run(command).await;
        }
        Commands::Totp { command } => {
            return commands::totp::run(command).await;
        }
        Commands::Update { yes, set_track } => {
            return commands::update::run(*yes, set_track.clone(), base_dir.clone()).await;
        }
        Commands::Support => {
            return commands::support::run().await;
        }
        _ => {}
    };

    let session = client.get_session(()).await;
    let (user_id, account_type) = match session {
        None => {
            return if cli.command.is_logout() {
                eprintln!("There was not an active session, you are already logged out");
                Ok(())
            } else {
                error!("Command is not logout there is no session");
                Err(anyhow!("This operation requires an authenticated client"))
            };
        }
        Some(session) => {
            if !session.is_authenticated().await {
                error!("Session is some but is not logged in");
                commands::logout::cleanup().await?;
                return Err(anyhow!("This operation requires an authenticated client"));
            }
            // Check if session needs extra password and get account type
            let (needs_extra_password, user_id, account_type) = {
                let store_guard = store.read();
                let needs_extra_password = store_guard.needs_extra_password();
                let auth = store_guard.auth.lock();
                let user_id = auth
                    .as_ref()
                    .and_then(|a| a.user_id().map(|u| u.to_string()));
                let account_type = store_guard.account_type();

                (needs_extra_password, user_id, account_type)
            };
            if needs_extra_password {
                error!("Session is some but needs extra password");
                commands::logout::cleanup().await?;
                return Err(anyhow!("This operation requires an authenticated client"));
            }
            (user_id, account_type)
        }
    };

    client_features.set_user_id(user_id.clone()).await;
    info!("Creating client with AccountType: {account_type:?}");

    let sdk = utils::create_sdk()?;
    let client = PassClient::new(client, client_features.clone(), account_type, sdk);
    pass::bootstrap_event_sync(&client).await;
    client_features
        .telemetry_handler
        .send_telemetry_if_needed(user_id, &client)
        .await;

    match cli.command {
        Commands::Logout { .. } => commands::logout::run(client).await,
        Commands::Test => commands::test::run(client).await,
        Commands::Info { output } => commands::info::run(client, base_dir, output).await,
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
        Commands::Session { command } => commands::session::run(command, client, store).await,
        Commands::User { command } => commands::user::run(command, client, store).await,
        Commands::SshAgent { command } => commands::ssh_agent::run(command, client, store).await,
        Commands::Settings { command } => commands::settings::run(command, client).await,
        Commands::PersonalAccessToken { command } => {
            commands::personal_access_token::run(command, client).await
        }
        Commands::Agent { command } => commands::agent::run(command, client).await,
        #[cfg(feature = "internal")]
        Commands::Internal { command } => {
            commands::internal::run(command, client, client_features).await
        }
        _ => Ok(()),
    }
}
