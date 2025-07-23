use crate::crypto::constants::SIGNATURE_CONTEXT_EXISTING_USER;
use crate::{PgpCrypto, PrivateKey, PublicKey, UnlockedAddressKeys, UserKey};
use anyhow::{Context, Result};
use std::sync::Arc;

pub(crate) struct InviteKeyToReencrypt {
    pub key: Vec<u8>,
    pub key_rotation: u8,
}

pub(crate) struct ReencryptedInviteKey {
    pub encrypted_key: Vec<u8>,
    pub key_rotation: u8,
}

pub(crate) struct ReencryptInviteKeysFlow {
    crypto: Arc<dyn PgpCrypto>,
    user_keys: Vec<UserKey>,
    user_address_keys: UnlockedAddressKeys,
    inviter_keys: Vec<PublicKey>,
}

impl ReencryptInviteKeysFlow {
    pub fn new(
        crypto: Arc<dyn PgpCrypto>,
        user_keys: Vec<UserKey>,
        user_address_keys: UnlockedAddressKeys,
        inviter_keys: Vec<PublicKey>,
    ) -> Self {
        Self {
            crypto,
            user_keys,
            user_address_keys,
            inviter_keys,
        }
    }

    pub async fn reencrypt(
        mut self,
        invite_keys: Vec<InviteKeyToReencrypt>,
    ) -> Result<Vec<ReencryptedInviteKey>> {
        let (user_private_key, user_public_key) = match self.user_keys.pop() {
            Some(k) => k.into_keys(),
            None => return Err(anyhow::anyhow!("User address key not found")),
        };

        let user_address_keys: Vec<PrivateKey> = self
            .user_address_keys
            .keys
            .into_values()
            .map(|k| k.private_key)
            .collect();

        let mut res = Vec::with_capacity(invite_keys.len());
        for invite_key in invite_keys {
            let rotation = invite_key.key_rotation;
            let decrypted = self
                .crypto
                .decrypt_and_verify(
                    invite_key.key,
                    user_address_keys.clone(),
                    self.inviter_keys.clone(),
                    Some(SIGNATURE_CONTEXT_EXISTING_USER.to_string()),
                )
                .await
                .context("Error decrypting invite key")?;

            let encrypted = self
                .crypto
                .encrypt_and_sign(
                    decrypted,
                    user_public_key.clone(),
                    user_private_key.clone(),
                    None,
                )
                .await
                .context("Error encrypting invite key")?;

            res.push(ReencryptedInviteKey {
                encrypted_key: encrypted,
                key_rotation: rotation,
            })
        }

        Ok(res)
    }
}
