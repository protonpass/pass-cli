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

use crate::{DataToDecrypt, Passphrase, PlainText, PrivateKey, PublicKey};
use anyhow::Result;

#[derive(Clone, Debug)]
pub enum DataEncoding {
    Armored,
    Binary,
}

#[derive(Clone, Debug)]
pub enum DataToArmor {
    Message(Vec<u8>),
    Signature(Vec<u8>),
    PrivateKey(Vec<u8>),
    PublicKey(Vec<u8>),
}

#[async_trait::async_trait]
pub trait PgpCrypto {
    async fn encrypt(&self, data: Vec<u8>, key: PublicKey) -> Result<Vec<u8>>;
    async fn encrypt_and_sign(
        &self,
        data: PlainText,
        encryption_key: PublicKey,
        signing_key: PrivateKey,
        signing_context: Option<String>,
    ) -> Result<Vec<u8>>;

    async fn sign(&self, data: Vec<u8>, signing_key: PrivateKey) -> Result<Vec<u8>>;

    async fn decrypt(&self, data: Vec<u8>, keys: Vec<PrivateKey>) -> Result<Vec<u8>>;
    async fn decrypt_and_verify(
        &self,
        data: Vec<u8>,
        decryption_keys: Vec<PrivateKey>,
        verification_keys: Vec<PublicKey>,
        verification_context: Option<String>,
    ) -> Result<Vec<u8>>;
    async fn decrypt_and_verify_data(
        &self,
        data: DataToDecrypt,
        decryption_keys: Vec<PrivateKey>,
        verification_keys: Vec<PublicKey>,
        verification_context: Option<String>,
    ) -> Result<Vec<u8>>;

    async fn armor(&self, data: DataToArmor) -> Result<String>;
    async fn unarmor(&self, armored: String) -> Result<Vec<u8>>;

    async fn open_private_key(&self, key: PrivateKey, passphrase: Passphrase)
    -> Result<PrivateKey>;
    async fn get_public_key(&self, key: PrivateKey) -> Result<PublicKey>;
    async fn generate_key_pair(
        &self,
        name: String,
        email: String,
    ) -> Result<(PrivateKey, PublicKey)>;
}
