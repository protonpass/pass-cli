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

use crate::crypto::constants::SIGNATURE_CONTEXT_EXISTING_USER;
use anyhow::{Context, Result};
use pass_domain::{PgpCrypto, PlainText, PrivateKey, PublicKey, UnlockedAddressKeys, UserKey};
use std::sync::Arc;
use zeroize::ZeroizeOnDrop;

#[derive(ZeroizeOnDrop)]
pub(crate) struct InviteKeyToReencrypt {
    pub key: Vec<u8>,
    pub key_rotation: u8,
}

#[derive(ZeroizeOnDrop)]
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
        self,
        invite_keys: Vec<InviteKeyToReencrypt>,
    ) -> Result<Vec<ReencryptedInviteKey>> {
        let (user_private_key, user_public_key) = match self.user_keys.first().cloned() {
            Some(k) => k.into_keys(),
            None => return Err(anyhow::anyhow!("User address key not found")),
        };

        let user_address_keys: Vec<PrivateKey> = self
            .user_address_keys
            .value()
            .into_values()
            .map(|k| k.private_key.clone())
            .collect();

        let mut res = Vec::with_capacity(invite_keys.len());
        for invite_key in invite_keys {
            let rotation = invite_key.key_rotation;
            let decrypted = self
                .crypto
                .decrypt_and_verify(
                    invite_key.key.clone(),
                    user_address_keys.clone(),
                    self.inviter_keys.clone(),
                    Some(SIGNATURE_CONTEXT_EXISTING_USER.to_string()),
                )
                .await
                .context("Error decrypting invite key")?;

            let encrypted = self
                .crypto
                .encrypt_and_sign(
                    PlainText::new(decrypted),
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
