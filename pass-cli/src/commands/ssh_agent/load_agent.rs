use super::VaultQuery;
use super::key_load;
use anyhow::{Context, Result, bail};
use pass::PassClient;
use pass_domain::TelemetryEvent;
use ssh_agent_client_rs::Client as SshAgentClient;
use ssh_key::private::PrivateKey as SshPrivateKey;
use std::path::PathBuf;

#[cfg(unix)]
fn get_system_agent_socket() -> Result<PathBuf> {
    let sock_path = std::env::var("SSH_AUTH_SOCK").context(
        "SSH_AUTH_SOCK environment variable is not set. Make sure the SSH agent is running.",
    )?;

    let path = PathBuf::from(sock_path);
    if !path.exists() {
        bail!(
            "SSH_AUTH_SOCK is set to '{}' but the socket does not exist",
            path.display()
        );
    }

    Ok(path)
}

#[cfg(windows)]
fn get_system_agent_socket() -> Result<PathBuf> {
    match std::env::var("SSH_AUTH_SOCK") {
        Ok(v) => Ok(PathBuf::from(v)),
        Err(_) => Ok(PathBuf::from(r"\\.\pipe\openssh-ssh-agent")),
    }
}

/// Add an identity to the SSH agent
fn add_identity_to_agent(client: &mut SshAgentClient, private_key: &SshPrivateKey) -> Result<()> {
    client
        .add_identity(private_key)
        .context("Failed to add identity to SSH agent")?;
    Ok(())
}

pub async fn run_load(
    share_id: Option<String>,
    vault_name: Option<String>,
    client: PassClient,
) -> Result<()> {
    client
        .emit_telemetry(TelemetryEvent::command("ssh-agent-load"))
        .await;
    let vault_query = VaultQuery::new(share_id, vault_name)?;

    // Get system SSH agent socket
    let socket_path = get_system_agent_socket().context("Failed to find system SSH agent")?;

    info!("Using SSH agent at: {}", socket_path.display());

    let mut agent_client =
        SshAgentClient::connect(&socket_path).context("Failed to connect to SSH agent")?;

    info!("Connected to SSH agent, Loading SSH keys");
    let identities = key_load::load_keys_into_storage(&client, &vault_query)
        .await
        .context("Failed to load SSH keys from vaults")?;

    if identities.is_empty() {
        bail!("No SSH keys found in the specified vault(s)");
    }

    let total_keys = identities.len();
    info!("Found {total_keys} SSH key(s)");

    // Get list of existing identities in the agent
    let existing_identities = agent_client
        .list_all_identities()
        .context("Failed to list existing identities in SSH agent")?;

    info!(
        "Found {} existing key(s) in SSH agent",
        existing_identities.len()
    );

    let mut success_count = 0;
    let mut failure_count = 0;
    let mut skipped_count = 0;

    for identity in identities {
        let comment = identity.comment.clone();
        let private_key = match identity.decrypt_private_key() {
            Ok(key) => key,
            Err(e) => {
                warn!("Failed to decrypt key '{comment}': {e}");
                failure_count += 1;
                continue;
            }
        };

        // Check if this key is already loaded by comparing public keys
        let public_key = private_key.public_key();
        let is_already_loaded = existing_identities.iter().any(|existing| match existing {
            ssh_agent_client_rs::Identity::PublicKey(existing_pk) => {
                existing_pk.key_data() == public_key.key_data()
            }
            _ => false,
        });

        if is_already_loaded {
            info!("Key '{comment}' is already loaded in SSH agent, skipping");
            skipped_count += 1;
            continue;
        }

        match add_identity_to_agent(&mut agent_client, &private_key) {
            Ok(_) => {
                info!("Successfully added key '{comment}' to SSH agent");
                success_count += 1;
            }
            Err(e) => {
                warn!("Failed to add key '{comment}' to SSH agent: {e}");
                failure_count += 1;
            }
        }
    }

    eprintln!("\nSSH Key Loading Summary:");
    eprintln!("  Successfully loaded: {}", success_count);
    if skipped_count > 0 {
        eprintln!("  Already loaded (skipped): {}", skipped_count);
    }
    if failure_count > 0 {
        eprintln!("  Failed to load: {}", failure_count);
    }
    eprintln!("  Total keys: {total_keys}");

    if success_count == 0 && skipped_count == 0 {
        bail!("Failed to load any SSH keys to the agent");
    }

    if success_count > 0 {
        eprintln!("\nKeys have been loaded into the system SSH agent.");
    } else {
        eprintln!("\nAll keys were already present in the system SSH agent.");
    }
    eprintln!("You can verify with: ssh-add -l");

    Ok(())
}
