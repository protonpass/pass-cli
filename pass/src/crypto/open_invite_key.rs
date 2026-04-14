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
