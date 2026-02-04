use crate::crypto::constants::SIGNATURE_CONTEXT_EXISTING_USER;
use anyhow::{Context, Result};
use pass_domain::{PgpCrypto, PlainText, PublicKey, UnlockedAddressKeys};
use std::sync::Arc;
use zeroize::ZeroizeOnDrop;

#[derive(ZeroizeOnDrop)]
pub(crate) struct PreparedInviteKey {
    pub key: Vec<u8>,
    pub key_rotation: u8,
}

pub(crate) struct InviteKeyToPrepare {
    pub decrypted_key: Vec<u8>,
    pub key_rotation: u8,
}

pub(crate) struct EncryptInviteKeysFlow {
    crypto: Arc<dyn PgpCrypto>,
    user_address_keys: UnlockedAddressKeys,
    invited_keys: Vec<PublicKey>,
}

impl EncryptInviteKeysFlow {
    pub fn new(
        crypto: Arc<dyn PgpCrypto>,
        user_address_keys: UnlockedAddressKeys,
        invited_keys: Vec<PublicKey>,
    ) -> Self {
        Self {
            crypto,
            user_address_keys,
            invited_keys,
        }
    }

    pub async fn encrypt(
        self,
        invite_keys: Vec<InviteKeyToPrepare>,
    ) -> Result<Vec<PreparedInviteKey>> {
        let signing_key = match self.user_address_keys.value().first_entry() {
            Some(k) => k.get().clone(),
            None => return Err(anyhow::anyhow!("User address key not found")),
        };

        let invited_key = match self.invited_keys.first().cloned() {
            Some(k) => k,
            None => return Err(anyhow::anyhow!("Empty list of invited_keys")),
        };

        debug!("[create_invite] signing_key: {}", signing_key.id);

        let mut res = Vec::with_capacity(invite_keys.len());
        for invite_key in invite_keys {
            let rotation = invite_key.key_rotation;
            let encrypted = self
                .crypto
                .encrypt_and_sign(
                    PlainText::new(invite_key.decrypted_key),
                    invited_key.clone(),
                    signing_key.private_key.clone(),
                    Some(SIGNATURE_CONTEXT_EXISTING_USER.to_string()),
                )
                .await
                .context("Error encrypting invite key")?;

            res.push(PreparedInviteKey {
                key: encrypted,
                key_rotation: rotation,
            })
        }

        Ok(res)
    }
}
