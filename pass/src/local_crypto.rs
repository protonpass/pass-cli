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

use crate::{PassClient, PassClientContext};
use anyhow::{Context, Result, anyhow};
use pass_domain::LocalKey;
use pass_domain::crypto::EncryptionTag;

impl<C: PassClientContext> PassClient<C> {
    pub async fn encrypt_with_local_key(&self, data: &[u8]) -> Result<Vec<u8>> {
        let local_key = self.get_local_key().await?;
        match pass_domain::crypto::encrypt(data, local_key.as_ref(), EncryptionTag::Unknown) {
            Ok(encrypted_data) => Ok(encrypted_data),
            Err(e) => Err(anyhow!("Error encrypting data: {:?}", e)),
        }
    }

    pub async fn decrypt_with_local_key(&self, ciphertext: &[u8]) -> Result<Vec<u8>> {
        let local_key = self.get_local_key().await?;
        match pass_domain::crypto::decrypt(ciphertext, local_key.as_ref(), EncryptionTag::Unknown) {
            Ok(data) => Ok(data),
            Err(e) => Err(anyhow!("Error decrypting data: {:?}", e)),
        }
    }

    async fn get_local_key(&self) -> Result<LocalKey> {
        let provider = self
            .client_features
            .get_local_key_provider()
            .await
            .context("Error getting local key provider")?;
        let local_key = provider
            .get_key()
            .await
            .context("Error getting local key")?;

        Ok(local_key)
    }
}
