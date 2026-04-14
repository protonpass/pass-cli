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

use crate::{AddressKeyId, PrivateKey};
use std::collections::BTreeMap;

#[derive(Clone)]
pub struct UnlockedAddressKey {
    pub id: AddressKeyId,
    pub private_key: PrivateKey,
}

#[derive(Clone)]
pub struct UnlockedAddressKeys {
    pub(crate) keys: BTreeMap<AddressKeyId, UnlockedAddressKey>,
}

impl UnlockedAddressKeys {
    pub fn new(keys: Vec<UnlockedAddressKey>) -> UnlockedAddressKeys {
        let mut as_btree = BTreeMap::new();
        for key in keys {
            as_btree.insert(key.id.clone(), key);
        }
        Self { keys: as_btree }
    }

    pub fn keys(&self) -> &BTreeMap<AddressKeyId, UnlockedAddressKey> {
        &self.keys
    }
    pub fn value(self) -> BTreeMap<AddressKeyId, UnlockedAddressKey> {
        self.keys
    }

    pub fn first(&self) -> Option<&UnlockedAddressKey> {
        self.keys.values().next()
    }

    pub fn first_or_err(&self) -> anyhow::Result<&UnlockedAddressKey> {
        match self.first() {
            Some(key) => Ok(key),
            None => anyhow::bail!("No address keys available"),
        }
    }

    pub fn into_keys(self) -> Vec<PrivateKey> {
        self.keys.into_values().map(|k| k.private_key).collect()
    }
}
