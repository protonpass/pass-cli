use crate::crypto::constants::SIGNATURE_CONTEXT_EXISTING_USER;
use crate::invite::list::{DecryptedInviteKey, InviteKey, OpenedInviteKey};
use anyhow::{Context, Result};
use pass_domain::{PgpCrypto, PublicKey, UnlockedAddressKeys};
use std::sync::Arc;

pub(crate) struct OpenInviteKeyFlow {
    pub crypto: Arc<dyn PgpCrypto>,
    pub user_address_keys: UnlockedAddressKeys,
    pub inviter_keys: Vec<PublicKey>,
}

impl OpenInviteKeyFlow {
    pub fn new(
        crypto: Arc<dyn PgpCrypto>,
        user_address_keys: UnlockedAddressKeys,
        inviter_keys: Vec<PublicKey>,
    ) -> Self {
        Self {
            crypto,
            user_address_keys,
            inviter_keys,
        }
    }

    pub async fn open(self, invite_keys: Vec<InviteKey>) -> Result<Vec<OpenedInviteKey>> {
        let mut private_keys = vec![];

        for address_key in self.user_address_keys.value().into_values() {
            private_keys.push(address_key.private_key);
        }

        let mut res = Vec::with_capacity(invite_keys.len());
        for invite_key in invite_keys {
            let rotation = invite_key.key_rotation;
            let opened = self
                .crypto
                .decrypt_and_verify(
                    invite_key.key.0,
                    private_keys.clone(),
                    self.inviter_keys.clone(),
                    Some(SIGNATURE_CONTEXT_EXISTING_USER.to_string()),
                )
                .await
                .context("Error decrypting invite key")?;

            res.push(OpenedInviteKey {
                key: DecryptedInviteKey(opened),
                key_rotation: rotation,
            })
        }

        Ok(res)
    }
}
