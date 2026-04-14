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

use super::event_handler::SshAgentEventHandler;
use super::event_processor::SshEventProcessor;
use super::key_storage::{IdentitySource, KeyStorage};
use super::{SshIdentity, VaultQuery, get_default_socket_path};
use crate::helpers::CliPassClient as PassClient;
use ssh_key::private::KeypairData;
use ssh_key::{Algorithm, HashAlg, Signature};
use std::sync::Arc;

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
        let pipe_name = r"\\.\pipe\openssh-ssh-agent";

        eprintln!("SSH agent listening on: {}", pipe_name);
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

#[allow(unused_variables)]
fn print_agent_startup_message(socket_display: &str, refresh_interval: u64) {
    eprintln!("SSH agent started successfully!");
    eprintln!("To use this agent, run:");
    #[cfg(unix)]
    {
        eprintln!("  export SSH_AUTH_SOCK={}", socket_display);
    }
    #[cfg(windows)]
    {
        eprintln!(
            "  1. Open 'Services' (you can use the Windows search bar or press Win+R and enter 'services.msc'"
        );
        eprintln!("  2. Find 'OpenSSH Authentication Agent', right-click 'Properties'");
        eprintln!("  3. Set 'Startup type' to Disabled, and click OK");
        eprintln!("  4. Ensure the service status is 'Stopped'");
    }

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
    // Create event channel and handler
    let (tx, mut rx) = tokio::sync::mpsc::channel(10);
    let handler = Arc::new(SshAgentEventHandler::new(tx, refresh_interval));

    // Spawn event listener
    let client_clone = client.clone();
    let event_listener = tokio::spawn(async move {
        if let Err(e) = client_clone.listen_for_events(handler).await {
            error!("Event listener error: {e:#}");
        }
    });

    // Create event processor
    let processor =
        SshEventProcessor::new(client.clone(), vault_query.clone(), key_storage.clone());

    // Main select loop
    tokio::select! {
        result = listen(listener, key_storage) => {
            result.context("SSH agent error")?;
        }
        _ = async {
            while let Some(events) = rx.recv().await {
                if let Err(e) = processor.process_events(events).await {
                    warn!("Error processing events: {}", e);
                }
            }
        } => {}
        _ = event_listener => {
            info!("Event listener task finished");
        }
        _ = tokio::signal::ctrl_c() => {
            eprintln!("Received Ctrl+C, shutting down...");
        }
    }

    Ok(())
}

#[ssh_agent_lib::async_trait]
impl Session for KeyStorage {
    async fn request_identities(&mut self) -> Result<Vec<message::Identity>, AgentError> {
        debug!("List identities request");
        let mut identities = vec![];
        for identity in self.identities.lock().await.iter() {
            // For now, always return the regular public key
            // Certificates are handled during signing, not in identity listing
            identities.push(message::Identity {
                pubkey: identity.pubkey_data.clone(),
                comment: identity.comment.clone(),
            })
        }
        Ok(identities)
    }

    async fn sign(&mut self, sign_request: SignRequest) -> Result<Signature, AgentError> {
        let pubkey: SshPublicKey = sign_request.pubkey.clone().into();

        debug!(
            "Sign request for public key: {}",
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
                error!("Failed to decrypt private key: {e:#}");
                std::io::Error::other(format!("Failed to decrypt private key: {}", e))
            })?;

            match private_key.key_data() {
                KeypairData::Rsa(key) => {
                    use rsa::signature::{RandomizedSigner, SignatureEncoding};
                    let algorithm;

                    // Work around a bug in ssh-key 0.6.7 where TryFrom<&RsaKeypair> for
                    // rsa::RsaPrivateKey uses `key.private.p` for both prime slots instead of
                    // `key.private.p` and `key.private.q`. This causes p*p ≠ n, making
                    // validation fail for all RSA keys - most visibly RSA 4096-bit keys.
                    let private_key = rsa::RsaPrivateKey::from_components(
                        rsa::BigUint::try_from(&key.public.n).map_err(AgentError::other)?,
                        rsa::BigUint::try_from(&key.public.e).map_err(AgentError::other)?,
                        rsa::BigUint::try_from(&key.private.d).map_err(AgentError::other)?,
                        vec![
                            rsa::BigUint::try_from(&key.private.p).map_err(AgentError::other)?,
                            rsa::BigUint::try_from(&key.private.q).map_err(AgentError::other)?,
                        ],
                    )
                    .map_err(AgentError::other)?;
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
        match identity.credential {
            Credential::Key { privkey, comment } => {
                let privkey = SshPrivateKey::try_from(privkey).map_err(AgentError::other)?;
                let identity =
                    SshIdentity::new(privkey, comment, IdentitySource::User).map_err(|e| {
                        std::io::Error::other(format!("Failed to create identity: {}", e))
                    })?;
                self.identity_add(identity).await;
                Ok(())
            }
            Credential::Cert {
                algorithm,
                certificate,
                comment,
                ..
            } => {
                info!(
                    "Adding certificate: [key_id={}] [certificate comment={}] [comment={}] [algorithm={}]",
                    certificate.key_id(),
                    certificate.comment(),
                    comment,
                    algorithm.as_str()
                );

                // Get the public key from the certificate
                let cert_public_key = ssh_key::PublicKey::from(certificate.public_key().clone());

                // Find the existing identity with this public key
                let mut identities = self.identities.lock().await;
                if let Some(identity) = identities
                    .iter_mut()
                    .find(|id| id.public_key.key_data() == cert_public_key.key_data())
                {
                    // Update the existing identity with the certificate
                    // The pubkey_data will be dynamically generated in request_identities
                    let cert_key_id = certificate.key_id().to_string();
                    identity.certificate = Some(*certificate);

                    info!(
                        "Certificate {} associated with existing key {}",
                        cert_key_id, identity.comment
                    );
                } else {
                    warn!(
                        "Certificate added but no matching key found (key needs to be added after certificate)"
                    );
                }

                Ok(())
            }
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
        info!(
            "Received a remove_identity request for pubkey: {}",
            pubkey.fingerprint(HashAlg::Sha256)
        );
        self.identity_remove(&pubkey, true).await?;
        Ok(())
    }

    async fn remove_all_identities(&mut self) -> Result<(), AgentError> {
        info!("Received a remove_all_identities request");
        self.replace_all_identities(Vec::new()).await;
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

#[cfg(test)]
mod tests {
    use ssh_key::{private::KeypairData, private::PrivateKey as SshPrivateKey};

    // Unencrypted RSA 4096 test key (generated for testing only, not used in production)
    const TEST_RSA_4096_KEY: &str = "-----BEGIN OPENSSH PRIVATE KEY-----
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
-----END OPENSSH PRIVATE KEY-----
";

    /// Regression test for RSA 4096 signing failure.
    ///
    /// ssh-key 0.6.7 has a bug in `TryFrom<&RsaKeypair> for rsa::RsaPrivateKey` where
    /// `key.private.p` is used for both prime slots instead of `p` and `q`. This causes
    /// `from_components` to fail (p*p ≠ n) for all RSA keys. Our signing code works around
    /// this by constructing the key manually with the correct `p` and `q`.
    #[test]
    fn test_rsa_4096_key_construction_succeeds() {
        let private_key =
            SshPrivateKey::from_openssh(TEST_RSA_4096_KEY).expect("Should parse RSA 4096 key");

        let KeypairData::Rsa(keypair) = private_key.key_data() else {
            panic!("Expected RSA keypair");
        };

        // The buggy try_into() would fail here for RSA 4096 keys.
        let buggy_result = rsa::RsaPrivateKey::try_from(keypair);
        assert!(
            buggy_result.is_err(),
            "ssh-key 0.6.7 bug: try_into() should fail due to p used twice instead of p and q"
        );

        // Our fix: construct from_components with the correct p and q.
        let fixed_result = rsa::RsaPrivateKey::from_components(
            rsa::BigUint::try_from(&keypair.public.n).expect("n"),
            rsa::BigUint::try_from(&keypair.public.e).expect("e"),
            rsa::BigUint::try_from(&keypair.private.d).expect("d"),
            vec![
                rsa::BigUint::try_from(&keypair.private.p).expect("p"),
                rsa::BigUint::try_from(&keypair.private.q).expect("q"),
            ],
        );
        assert!(
            fixed_result.is_ok(),
            "Fixed construction should succeed: {:?}",
            fixed_result.err()
        );
    }
}
