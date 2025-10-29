use anyhow::{Context, Result};
use rsa::pkcs1v15::SigningKey;
use sha1::Sha1;
use sha2::{Sha256, Sha512};
use ssh_agent_lib::agent::{ListeningSocket, Session, listen};
use ssh_agent_lib::error::AgentError;
use ssh_agent_lib::proto::extension::{QueryResponse, SessionBind};
use ssh_agent_lib::proto::{
    AddIdentity, AddIdentityConstrained, AddSmartcardKeyConstrained, Credential, Extension,
    RemoveIdentity, SignRequest, SmartcardKey, message,
};
use ssh_key::{private::PrivateKey as SshPrivateKey, public::PublicKey as SshPublicKey};
use std::path::PathBuf;

use super::key_storage::KeyStorage;
use super::{Identity, VaultQuery, get_default_socket_path};
use crate::commands::ssh_agent::key_load::refresh_keys_periodically;
use pass::PassClient;
use ssh_key::private::KeypairData;
use ssh_key::{Algorithm, HashAlg, Signature};

pub async fn start_agent(
    client: &PassClient,
    vault_query: &VaultQuery,
    key_storage: KeyStorage,
    socket_path: Option<String>,
    refresh_interval: u64,
) -> Result<()> {
    let socket_path = if let Some(path) = socket_path {
        PathBuf::from(path)
    } else {
        get_default_socket_path()?
    };

    if refresh_interval > 0 {
        info!(
            "Automatic key refresh enabled (every {} seconds)",
            refresh_interval
        );
    } else {
        info!("Automatic key refresh disabled");
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        use tokio::net::UnixListener;

        // Remove existing socket if it exists
        if socket_path.exists() {
            std::fs::remove_file(&socket_path).context("Failed to remove existing socket file")?;
        }

        // Ensure parent directory exists
        if let Some(parent) = socket_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .context("Failed to create socket directory")?;
        }

        // Create Unix socket
        let listener = UnixListener::bind(&socket_path).context("Failed to bind Unix socket")?;

        // Set socket permissions to 0600 (owner read/write only)
        let metadata = std::fs::metadata(&socket_path).context("Failed to get socket metadata")?;
        let mut permissions = metadata.permissions();
        permissions.set_mode(0o600);
        std::fs::set_permissions(&socket_path, permissions)
            .context("Failed to set socket permissions")?;

        info!("SSH agent listening on: {}", socket_path.display());
        print_agent_startup_message(&socket_path.display().to_string(), refresh_interval);

        let socket_path_clone = socket_path.clone();

        // Run the agent
        run_agent_with_listener(listener, key_storage, refresh_interval, client, vault_query)
            .await?;

        // Cleanup
        if socket_path_clone.exists() {
            std::fs::remove_file(&socket_path_clone).context("Failed to remove socket file")?;
        }
    }

    #[cfg(windows)]
    {
        use ssh_agent_lib::agent::NamedPipeListener;

        // On Windows, use a named pipe
        let username = std::env::var("USERNAME").unwrap_or_else(|_| "user".to_string());
        let pipe_name = format!(r"\\.\pipe\proton-pass-agent-{}", username);

        info!("SSH agent listening on: {}", pipe_name);
        print_agent_startup_message(&socket_path.display().to_string(), refresh_interval);

        let listener = NamedPipeListener::bind(&pipe_name).context("Failed to bind named pipe")?;

        // Run the agent
        run_agent_with_listener(
            listener,
            key_storage,
            refresh_interval,
            &client,
            &vault_query,
        )
        .await?;
    }

    eprintln!("SSH agent stopped.");

    Ok(())
}

fn print_agent_startup_message(socket_display: &str, refresh_interval: u64) {
    eprintln!("SSH agent started successfully!");
    eprintln!("To use this agent, run:");
    #[cfg(unix)]
    eprintln!("  export SSH_AUTH_SOCK={}", socket_display);
    #[cfg(windows)]
    eprintln!("  $env:SSH_AUTH_SOCK = '{}'", socket_display);

    if refresh_interval > 0 {
        eprintln!(
            "\nKeys will refresh automatically every {} seconds.",
            refresh_interval
        );
    }
    eprintln!("\nPress Ctrl+C to stop the agent.");
}

async fn run_agent_with_listener<L>(
    listener: L,
    key_storage: KeyStorage,
    refresh_interval: u64,
    client: &PassClient,
    vault_query: &VaultQuery,
) -> Result<()>
where
    L: ListeningSocket + Send + std::fmt::Debug,
    KeyStorage: ssh_agent_lib::agent::Agent<L>,
{
    if refresh_interval > 0 {
        let mut refresh_interval_timer =
            tokio::time::interval(tokio::time::Duration::from_secs(refresh_interval));
        refresh_interval_timer.tick().await; // Skip the first immediate tick

        tokio::select! {
            result = listen(listener, key_storage.clone()) => {
                result.context("SSH agent error")?;
            }
            _ = async {
                loop {
                    refresh_interval_timer.tick().await;
                    refresh_keys_periodically(client, vault_query, &key_storage, refresh_interval).await;
                }
            } => {}
            _ = tokio::signal::ctrl_c() => {
                info!("Received Ctrl+C, shutting down...");
            }
        }
    } else {
        tokio::select! {
            result = listen(listener, key_storage) => {
                result.context("SSH agent error")?;
            }
            _ = tokio::signal::ctrl_c() => {
                info!("Received Ctrl+C, shutting down...");
            }
        }
    }

    Ok(())
}

#[ssh_agent_lib::async_trait]
impl Session for KeyStorage {
    async fn request_identities(&mut self) -> Result<Vec<message::Identity>, AgentError> {
        let mut identities = vec![];
        for identity in self.identities.lock().await.iter() {
            identities.push(message::Identity {
                pubkey: identity.public_key.key_data().clone(),
                comment: identity.comment.clone(),
            })
        }
        Ok(identities)
    }

    async fn sign(&mut self, sign_request: SignRequest) -> Result<Signature, AgentError> {
        let pubkey: SshPublicKey = sign_request.pubkey.clone().into();

        debug!(
            "Sign request for public key: {:?}",
            pubkey.fingerprint(HashAlg::Sha256)
        );

        // Log all available identities for debugging
        {
            let identities = self.identities.lock().await;
            debug!("Available identities: {}", identities.len());
            for (idx, id) in identities.iter().enumerate() {
                debug!(
                    "  Identity {}: {} - {:?}",
                    idx,
                    id.comment,
                    id.public_key.fingerprint(HashAlg::Sha256)
                );
            }
        }

        if let Some(identity) = self.identity_from_pubkey(&pubkey).await {
            debug!("Found matching identity: {}", identity.comment);

            // Decrypt the private key on-demand
            let private_key = identity.decrypt_private_key().map_err(|e| {
                error!("Failed to decrypt private key: {}", e);
                std::io::Error::other(format!("Failed to decrypt private key: {}", e))
            })?;

            match private_key.key_data() {
                KeypairData::Rsa(key) => {
                    use rsa::signature::{RandomizedSigner, SignatureEncoding};
                    let algorithm;

                    let private_key: rsa::RsaPrivateKey =
                        key.try_into().map_err(AgentError::other)?;
                    let mut rng = rand::thread_rng();
                    let data = &sign_request.data;

                    let signature = if sign_request.flags
                        & ssh_agent_lib::proto::signature::RSA_SHA2_512
                        != 0
                    {
                        algorithm = "rsa-sha2-512";
                        SigningKey::<Sha512>::new(private_key).sign_with_rng(&mut rng, data)
                    } else if sign_request.flags & ssh_agent_lib::proto::signature::RSA_SHA2_256
                        != 0
                    {
                        algorithm = "rsa-sha2-256";
                        SigningKey::<Sha256>::new(private_key).sign_with_rng(&mut rng, data)
                    } else {
                        algorithm = "ssh-rsa";
                        SigningKey::<Sha1>::new(private_key).sign_with_rng(&mut rng, data)
                    };
                    Ok(Signature::new(
                        Algorithm::new(algorithm).map_err(AgentError::other)?,
                        signature.to_bytes().to_vec(),
                    )
                    .map_err(AgentError::other)?)
                }
                KeypairData::Ed25519(key) => {
                    use ed25519_dalek::{Signer, SigningKey as Ed25519SigningKey};
                    let signing_key = Ed25519SigningKey::from_bytes(&key.private.to_bytes());
                    let signature_bytes: ed25519_dalek::Signature =
                        signing_key.sign(&sign_request.data);

                    Ok(Signature::new(
                        Algorithm::new("ssh-ed25519").map_err(AgentError::other)?,
                        signature_bytes.to_bytes().to_vec(),
                    )
                    .map_err(AgentError::other)?)
                }
                KeypairData::Ecdsa(keypair) => {
                    use ssh_key::EcdsaCurve;

                    let (algorithm, signature_bytes) = match keypair.curve() {
                        EcdsaCurve::NistP256 => {
                            use p256::ecdsa::{SigningKey, signature::Signer};
                            use p256::elliptic_curve::generic_array::GenericArray;
                            let private_bytes = keypair.private_key_bytes();
                            let key_array = GenericArray::from_slice(private_bytes);
                            let signing_key =
                                SigningKey::from_bytes(key_array).map_err(AgentError::other)?;
                            let sig: p256::ecdsa::Signature = signing_key.sign(&sign_request.data);
                            ("ecdsa-sha2-nistp256", sig.to_bytes().to_vec())
                        }
                        EcdsaCurve::NistP384 => {
                            use p384::ecdsa::{SigningKey, signature::Signer};
                            use p384::elliptic_curve::generic_array::GenericArray;
                            let private_bytes = keypair.private_key_bytes();
                            let key_array = GenericArray::from_slice(private_bytes);
                            let signing_key =
                                SigningKey::from_bytes(key_array).map_err(AgentError::other)?;
                            let sig: p384::ecdsa::Signature = signing_key.sign(&sign_request.data);
                            ("ecdsa-sha2-nistp384", sig.to_bytes().to_vec())
                        }
                        EcdsaCurve::NistP521 => {
                            use p521::ecdsa::{SigningKey, signature::Signer};
                            use p521::elliptic_curve::generic_array::GenericArray;
                            let private_bytes = keypair.private_key_bytes();
                            let key_array = GenericArray::from_slice(private_bytes);
                            let signing_key =
                                SigningKey::from_bytes(key_array).map_err(AgentError::other)?;
                            let sig: p521::ecdsa::Signature = signing_key.sign(&sign_request.data);
                            ("ecdsa-sha2-nistp521", sig.to_bytes().to_vec())
                        }
                    };

                    Ok(Signature::new(
                        Algorithm::new(algorithm).map_err(AgentError::other)?,
                        signature_bytes,
                    )
                    .map_err(AgentError::other)?)
                }
                _ => Err(std::io::Error::other("Signature for key type not implemented").into()),
            }
        } else {
            error!("Failed to find identity for requested public key");
            Err(std::io::Error::other("Failed to create signature: identity not found").into())
        }
    }

    async fn add_identity(&mut self, identity: AddIdentity) -> Result<(), AgentError> {
        if let Credential::Key { privkey, comment } = identity.credential {
            let privkey = SshPrivateKey::try_from(privkey).map_err(AgentError::other)?;
            let identity = Identity::new(privkey, comment)
                .map_err(|e| std::io::Error::other(format!("Failed to create identity: {}", e)))?;
            self.identity_add(identity).await;
            Ok(())
        } else {
            info!("Unsupported key type: {:#?}", identity.credential);
            Ok(())
        }
    }

    async fn add_identity_constrained(
        &mut self,
        identity: AddIdentityConstrained,
    ) -> Result<(), AgentError> {
        let AddIdentityConstrained {
            identity,
            constraints,
        } = identity;
        info!("Would use these constraints: {constraints:#?}");
        self.add_identity(identity).await
    }

    async fn remove_identity(&mut self, identity: RemoveIdentity) -> Result<(), AgentError> {
        let pubkey: SshPublicKey = identity.pubkey.into();
        self.identity_remove(&pubkey).await?;
        Ok(())
    }

    async fn add_smartcard_key(&mut self, key: SmartcardKey) -> Result<(), AgentError> {
        info!("Adding smartcard key: {key:?}");
        Ok(())
    }

    async fn add_smartcard_key_constrained(
        &mut self,
        key: AddSmartcardKeyConstrained,
    ) -> Result<(), AgentError> {
        info!("Adding smartcard key with constraints: {key:?}");
        Ok(())
    }

    async fn lock(&mut self, _pwd: String) -> Result<(), AgentError> {
        info!("Locked with password");
        Ok(())
    }

    async fn unlock(&mut self, _pwd: String) -> Result<(), AgentError> {
        info!("Unlocked with password");
        Ok(())
    }

    async fn extension(&mut self, extension: Extension) -> Result<Option<Extension>, AgentError> {
        info!("Extension request: {}", extension.name);

        match extension.name.as_str() {
            "query" => {
                let response = Extension::new_message(QueryResponse {
                    extensions: vec!["query".into(), "session-bind@openssh.com".into()],
                })?;
                Ok(Some(response))
            }
            "session-bind@openssh.com" => {
                match extension.parse_message::<SessionBind>()? {
                    Some(bind) => {
                        // Verify the session binding signature
                        bind.verify_signature()
                            .map_err(|_| AgentError::ExtensionFailure)?;

                        info!("Session binding verified successfully");
                        Ok(None)
                    }
                    None => {
                        warn!("Failed to parse session-bind extension");
                        Err(AgentError::Failure)
                    }
                }
            }
            _ => {
                info!("Unsupported extension: {}", extension.name);
                Err(AgentError::Failure)
            }
        }
    }
}
