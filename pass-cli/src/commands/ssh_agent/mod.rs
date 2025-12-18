mod agent;
mod event_handler;
mod event_processor;
mod key_load;
mod key_storage;
mod load_agent;

use crate::telemetry::event::CommandEvent;
use anyhow::{Context, Result, anyhow};
use clap::Subcommand;
use key_storage::{KeyStorage, SshIdentity};
use pass::ssh_key::SshKeyItemCreatePayload;
use pass::{PassClient, is_id};
use pass_domain::{ItemId, PermissionFlag, ShareId};
use ssh_agent_lib::ssh_encoding::LineEnding;
use ssh_key::HashAlg;
use std::path::PathBuf;
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};

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
            help = "Interval in seconds to check for new SSH keys in Proton Pass",
            default_value = "30"
        )]
        refresh_interval: u64,
        #[arg(
            long,
            value_name = "VAULT_NAME_OR_SHARE_ID",
            help = "Automatically create new SSH key items in the specified vault when identities are added via ssh-add. Specify either a vault name or share ID."
        )]
        create_new_identities: Option<String>,
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
            create_new_identities,
        } => {
            run_start(
                socket_path,
                share_id,
                vault_name,
                refresh_interval,
                create_new_identities,
                client,
            )
            .await
        }
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
    create_new_identities: Option<String>,
    client: PassClient,
) -> Result<()> {
    client
        .emit_telemetry(&CommandEvent::new("ssh-agent-start"))
        .await;
    let vault_query = VaultQuery::new(share_id, vault_name)?;

    // Resolve the target share ID for creating new identities
    let create_target_share_id = if let Some(ref target) = create_new_identities {
        Some(resolve_vault_to_share_id(&client, target).await?)
    } else {
        None
    };

    info!("Loading SSH keys from Proton Pass...");
    eprintln!("Retrieving SSH keys from Proton Pass...");
    let identities = key_load::fetch_ssh_keys(&client, &vault_query)
        .await
        .context("Failed to load SSH keys from vaults")?;

    let loaded_count = identities.len();
    info!("Found {} SSH key(s)", loaded_count);

    // Set up the channel for item creation if auto-creation is enabled
    let (tx, rx) = unbounded_channel();

    let key_storage = KeyStorage::new(tx);
    key_storage.replace_all_identities(identities).await;

    if loaded_count == 0 {
        eprintln!("No SSH keys found in the specified vault(s)");
    } else {
        eprintln!("Loaded {} SSH key(s) successfully", loaded_count);
    }

    if create_new_identities.is_some() {
        eprintln!("Auto-creation of new SSH identities is enabled");
    }

    let listener_fut = listen_for_item_create_events(client.clone(), create_target_share_id, rx);
    let agent_fut = agent::start_agent(
        &client,
        &vault_query,
        key_storage,
        socket_path,
        refresh_interval,
    );

    tokio::select! {
        result = listener_fut => {
            info!("Item creation listener finished");
            result
        }
        result = agent_fut => {
            info!("SSH agent finished");
            result
        }
    }
}

async fn listen_for_item_create_events(
    client: PassClient,
    share_id: Option<ShareId>,
    mut rx: UnboundedReceiver<SshIdentity>,
) -> Result<()> {
    while let Some(identity) = rx.recv().await {
        info!("Received SSH key item create event");
        match share_id {
            Some(ref share_id) => {
                create_item_for_identity(&client, share_id, identity).await;
            }
            None => {
                info!("Not storing the ssh key in Pass because no target share is defined");
            }
        }
    }
    Ok(())
}

async fn create_item_for_identity(client: &PassClient, share_id: &ShareId, identity: SshIdentity) {
    match inner_create_item_for_identity(client, share_id, identity).await {
        Ok((item_id, title)) => eprintln!("Created a new item: {title} [{item_id}]"),
        Err(e) => {
            eprintln!("[ERROR] Failed to create new item for the new ssh key: {e:#}");
        }
    }
}

async fn inner_create_item_for_identity(
    client: &PassClient,
    share_id: &ShareId,
    identity: SshIdentity,
) -> Result<(ItemId, String)> {
    let private_key = identity
        .decrypt_private_key()
        .context("Error decrypting private key")?;
    let public_key = &identity.public_key;

    let title = if identity.comment.is_empty() {
        let fingerprint = public_key.fingerprint(HashAlg::Sha256).to_string();
        fingerprint
            .replace("SHA256:", "")
            .chars()
            .take(16)
            .collect::<String>()
    } else {
        identity.comment.to_string()
    };
    let title = format!("SSH Key {}", title);

    let private_key_as_openssh = private_key
        .to_openssh(LineEnding::CRLF)
        .context("Error formatting SSH private key")?;
    let public_key_as_openssh = public_key
        .to_openssh()
        .context("Error formatting SSH public key")?;

    let item_id = client
        .create_ssh_key(
            share_id,
            SshKeyItemCreatePayload {
                title: title.to_string(),
                private_key: private_key_as_openssh.to_string(),
                public_key: public_key_as_openssh,
                passphrase: None,
            },
            None,
        )
        .await
        .context("Failed to create SSH key item")?;
    Ok((item_id, title))
}

async fn resolve_vault_to_share_id(client: &PassClient, target: &str) -> Result<ShareId> {
    if is_id(target) {
        let share_id = ShareId::new(target.to_string());
        let share = client
            .get_share(&share_id)
            .await
            .context("Failed to get share")?;
        if share.has_permission(PermissionFlag::Create) {
            Ok(share_id)
        } else {
            Err(anyhow!(
                "The specified share does not allow you to create new items"
            ))
        }
    } else {
        let vault = client
            .find_vault(target)
            .await
            .context(format!("Failed to find vault with name '{}'", target))?;

        Ok(vault.share_id)
    }
}
