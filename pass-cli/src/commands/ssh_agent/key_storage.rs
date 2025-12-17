use anyhow::anyhow;
use ssh_agent_lib::error::AgentError;
use ssh_key::public::KeyData;
use ssh_key::{
    certificate::Certificate, private::PrivateKey as SshPrivateKey,
    public::PublicKey as SshPublicKey,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc::UnboundedSender;

#[derive(Clone, PartialEq, Debug)]
pub enum IdentitySource {
    ProtonPass,
    User,
}

#[derive(Clone, PartialEq, Debug)]
pub struct SshIdentity {
    pub public_key: SshPublicKey,
    pub encrypted_private_key_bytes: Vec<u8>,
    pub xor_key: u8,
    pub comment: String,
    pub source: IdentitySource,
    pub certificate: Option<Certificate>,

    // The pubkey data to return in SSH_AGENTC_REQUEST_IDENTITIES
    // For certificates, this contains the certificate blob
    // For regular keys, this contains the public key data
    pub pubkey_data: KeyData,
}

impl SshIdentity {
    pub fn new(
        private_key: SshPrivateKey,
        comment: String,
        source: IdentitySource,
    ) -> anyhow::Result<Self> {
        let public_key = SshPublicKey::from(&private_key);
        let pubkey_data = public_key.key_data().clone();
        Self::new_with_pubkey_data(private_key, pubkey_data, comment, source, None)
    }

    fn new_with_pubkey_data(
        private_key: SshPrivateKey,
        pubkey_data: KeyData,
        comment: String,
        source: IdentitySource,
        certificate: Option<Certificate>,
    ) -> anyhow::Result<Self> {
        let public_key = SshPublicKey::from(&private_key);
        let xor_key = pass_domain::crypto::generate_random_byte();

        let private_key_bytes = private_key
            .to_bytes()
            .map_err(|e| anyhow!("Failed to serialize private key: {}", e))?;

        let encrypted_private_key_bytes = Self::xor_bytes(&private_key_bytes, xor_key);

        Ok(Self {
            public_key,
            encrypted_private_key_bytes,
            xor_key,
            comment,
            source,
            certificate,
            pubkey_data,
        })
    }

    pub fn decrypt_private_key(&self) -> anyhow::Result<SshPrivateKey> {
        let decrypted_bytes = Self::xor_bytes(&self.encrypted_private_key_bytes, self.xor_key);

        SshPrivateKey::from_bytes(&decrypted_bytes)
            .map_err(|e| anyhow!("Failed to deserialize private key: {}", e))
    }

    fn xor_bytes(data: &[u8], xor_key: u8) -> Vec<u8> {
        data.iter().map(|b| b ^ xor_key).collect()
    }
}

#[derive(Clone)]
pub struct KeyStorage {
    pub identities: Arc<Mutex<Vec<SshIdentity>>>,
    pub create_item_sender: UnboundedSender<SshIdentity>,
}

impl KeyStorage {
    pub fn new(create_item_sender: UnboundedSender<SshIdentity>) -> Self {
        Self {
            identities: Arc::new(Mutex::new(Vec::new())),
            create_item_sender,
        }
    }

    pub async fn identity_from_pubkey(&self, pubkey: &SshPublicKey) -> Option<SshIdentity> {
        let identities = self.identities.lock().await;

        let index = Self::identity_index_from_pubkey(&identities, pubkey)?;
        Some(identities[index].clone())
    }

    pub async fn identity_add(&self, identity: SshIdentity) {
        let mut identities = self.identities.lock().await;
        if Self::identity_index_from_pubkey(&identities, &identity.public_key).is_none() {
            if let Err(e) = self.create_item_sender.send(identity.clone()) {
                warn!("Failed to send identity add: {}", e);
            }
            identities.push(identity);
        }
    }

    pub async fn identity_remove(&self, pubkey: &SshPublicKey) -> anyhow::Result<(), AgentError> {
        let mut identities = self.identities.lock().await;

        if let Some(index) = Self::identity_index_from_pubkey(&identities, pubkey) {
            identities.remove(index);
            Ok(())
        } else {
            Err(std::io::Error::other("Failed to remove identity: identity not found").into())
        }
    }

    pub async fn replace_all_identities(&self, new_identities: Vec<SshIdentity>) {
        let mut self_identities = self.identities.lock().await;

        let mut final_identities = HashMap::new();

        // Keep identities added manually by the user
        let user_added_identities: Vec<SshIdentity> = self_identities
            .iter()
            .filter(|i| i.source == IdentitySource::User)
            .cloned()
            .collect();
        for identity in user_added_identities {
            final_identities.insert(identity.public_key.key_data().clone(), identity);
        }

        // Add the new identities, using the hashmap to ensure we don't duplicate them
        for identity in new_identities {
            final_identities.insert(identity.public_key.key_data().clone(), identity);
        }

        let identities: Vec<SshIdentity> = final_identities.into_values().collect();
        *self_identities = identities;
    }

    fn identity_index_from_pubkey(
        identities: &[SshIdentity],
        pubkey: &SshPublicKey,
    ) -> Option<usize> {
        // Compare by key data instead of the full PublicKey object, since metadata might differ
        let target_key_data = pubkey.key_data();
        for (index, identity) in identities.iter().enumerate() {
            if identity.public_key.key_data() == target_key_data {
                return Some(index);
            }
        }
        None
    }
}
