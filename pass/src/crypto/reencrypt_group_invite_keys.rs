use crate::crypto::constants::SIGNATURE_CONTEXT_EXISTING_USER;
use crate::{PgpCrypto, PlainText, PublicKey, UnlockedAddressKeys};
use anyhow::{Context, Result, anyhow};
use std::sync::Arc;
use zeroize::ZeroizeOnDrop;

#[derive(ZeroizeOnDrop)]
pub(crate) struct GroupInviteKeyToReencrypt {
    pub key: Vec<u8>,
    pub key_rotation: u8,
}

#[derive(ZeroizeOnDrop)]
pub(crate) struct ReencryptedGroupInviteKey {
    pub encrypted_key: Vec<u8>,
    pub key_rotation: u8,
}

pub(crate) struct ReencryptGroupInviteKeysFlow {
    crypto: Arc<dyn PgpCrypto>,
    address_keys: UnlockedAddressKeys,
    inviter_keys: Vec<PublicKey>,
}

impl ReencryptGroupInviteKeysFlow {
    pub fn new(
        crypto: Arc<dyn PgpCrypto>,
        address_keys: UnlockedAddressKeys,
        inviter_keys: Vec<PublicKey>,
    ) -> Self {
        Self {
            crypto,
            address_keys,
            inviter_keys,
        }
    }

    pub async fn reencrypt(
        self,
        invite_keys: Vec<GroupInviteKeyToReencrypt>,
    ) -> Result<Vec<ReencryptedGroupInviteKey>> {
        let mut group_address_public_keys = Vec::with_capacity(self.address_keys.keys.len());
        let mut group_address_private_keys = Vec::with_capacity(self.address_keys.keys.len());
        for key in self.address_keys.keys.into_values() {
            let public_key = self
                .crypto
                .get_public_key(key.private_key.clone())
                .await
                .context("Error getting public key from private key")?;

            group_address_public_keys.push(public_key);
            group_address_private_keys.push(key.private_key);
        }

        let mut res = Vec::with_capacity(invite_keys.len());
        for invite_key in invite_keys {
            let rotation = invite_key.key_rotation;
            let decrypted = self
                .crypto
                .decrypt_and_verify(
                    invite_key.key.clone(),
                    group_address_private_keys.clone(),
                    self.inviter_keys.clone(),
                    Some(SIGNATURE_CONTEXT_EXISTING_USER.to_string()),
                )
                .await
                .context("Error decrypting invite key")?;

            let public_key = match group_address_public_keys.first() {
                Some(k) => k.clone(),
                None => return Err(anyhow!("Empty public key list")),
            };
            let private_key = match group_address_private_keys.first() {
                Some(k) => k.clone(),
                None => return Err(anyhow!("Empty private key list")),
            };

            let encrypted = self
                .crypto
                .encrypt_and_sign(PlainText(decrypted), public_key, private_key, None)
                .await
                .context("Error encrypting group invite key")?;

            res.push(ReencryptedGroupInviteKey {
                encrypted_key: encrypted,
                key_rotation: rotation,
            })
        }

        Ok(res)
    }
}
