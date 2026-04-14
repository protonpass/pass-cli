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

use crate::{
    AddressKey, KeySalt, LockedUserKey, Passphrase, PrivateKey, PublicKey, UnlockedAddressKeys,
    UserKey,
};
use anyhow::Result;
use std::collections::HashMap;

#[async_trait::async_trait]
pub trait AccountCrypto {
    async fn generate_passphrases(
        &self,
        key_salts: Vec<KeySalt>,
        pass: &str,
    ) -> Result<HashMap<String, Passphrase>>;

    async fn open_user_keys(
        &self,
        keys: Vec<LockedUserKey>,
        passphrases: HashMap<String, Passphrase>,
    ) -> Result<Vec<UserKey>>;

    async fn open_address_keys(
        &self,
        user_keys: Vec<UserKey>,
        address_keys: Vec<AddressKey>,
    ) -> Result<UnlockedAddressKeys>;

    async fn open_address_keys_with_keys(
        &self,
        private_keys: Vec<PrivateKey>,
        public_keys: Vec<PublicKey>,
        address_keys: Vec<AddressKey>,
    ) -> Result<UnlockedAddressKeys>;
}
