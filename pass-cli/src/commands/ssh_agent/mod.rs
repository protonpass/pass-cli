mod agent;
mod key_load;
mod key_storage;
mod load_agent;

use anyhow::{Context, Result, anyhow, bail};
use clap::Subcommand;
use key_storage::{Identity, KeyStorage};
use pass::PassClient;
use pass_domain::ShareId;
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum SshAgentCommands {
    #[command(about = "Start a Proton Pass SSH agent")]
    Start {
        #[arg(
            long,
            help = "Path to the SSH agent socket (Unix) or named pipe identifier (Windows)"
        )]
        socket_path: Option<String>,
        #[arg(long, help = "Share ID of the vault to load keys from")]
        share_id: Option<String>,
        #[arg(long, help = "Name of the vault to load keys from")]
        vault_name: Option<String>,
        #[arg(
            long,
            help = "Interval in seconds to refresh SSH keys from Proton Pass (0 to disable)",
            default_value = "3600"
        )]
        refresh_interval: u64,
    },
    #[command(about = "Load SSH keys from Proton Pass into the system SSH agent")]
    Load {
        #[arg(long, help = "Share ID of the vault to load keys from")]
        share_id: Option<String>,
        #[arg(long, help = "Name of the vault to load keys from")]
        vault_name: Option<String>,
    },
}

#[derive(Clone)]
pub enum VaultQuery {
    ShareId(ShareId),
    VaultName(String),
    All,
}

impl VaultQuery {
    pub fn new(share_id: Option<String>, vault_name: Option<String>) -> Result<Self> {
        match (share_id, vault_name) {
            (Some(share_id), None) => Ok(Self::ShareId(ShareId::new(share_id))),
            (None, Some(vault_name)) => Ok(Self::VaultName(vault_name)),
            (None, None) => Ok(Self::All),
            (Some(_), Some(_)) => Err(anyhow!(
                "Please provide either --share-id or --vault-name, not both"
            )),
        }
    }
}

#[cfg(unix)]
fn get_default_socket_path() -> Result<PathBuf> {
    let home_dir = dirs::home_dir().ok_or_else(|| anyhow!("Could not determine home directory"))?;
    Ok(home_dir.join(".ssh").join("proton-pass-agent.sock"))
}

#[cfg(windows)]
fn get_default_socket_path() -> Result<PathBuf> {
    let home_dir = dirs::home_dir().ok_or_else(|| anyhow!("Could not determine home directory"))?;
    // On Windows, we'll use the path for reference, but actual pipe name is different
    Ok(home_dir.join(".ssh").join("proton-pass-agent"))
}

pub async fn run(command: SshAgentCommands, client: PassClient) -> Result<()> {
    match command {
        SshAgentCommands::Start {
            socket_path,
            share_id,
            vault_name,
            refresh_interval,
        } => run_start(socket_path, share_id, vault_name, refresh_interval, client).await,
        SshAgentCommands::Load {
            share_id,
            vault_name,
        } => load_agent::run_load(share_id, vault_name, client).await,
    }
}

async fn run_start(
    socket_path: Option<String>,
    share_id: Option<String>,
    vault_name: Option<String>,
    refresh_interval: u64,
    client: PassClient,
) -> Result<()> {
    let vault_query = VaultQuery::new(share_id, vault_name)?;

    info!("Loading SSH keys from Proton Pass...");

    let identities = key_load::load_keys_into_storage(&client, &vault_query)
        .await
        .context("Failed to load SSH keys from vaults")?;

    if identities.is_empty() {
        bail!("No SSH keys found in the specified vault(s)");
    }

    let loaded_count = identities.len();
    info!("Found {} SSH key(s)", loaded_count);

    let key_storage = KeyStorage::default();
    key_storage.replace_all_identities(identities).await;

    eprintln!("Loaded {} SSH key(s) successfully", loaded_count);

    agent::start_agent(
        &client,
        &vault_query,
        key_storage,
        socket_path,
        refresh_interval,
    )
    .await
}
