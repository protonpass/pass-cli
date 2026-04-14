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
