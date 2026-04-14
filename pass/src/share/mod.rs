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

pub(crate) mod keys;
pub(crate) mod list;
mod open_key;

use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Clone, Debug, Eq, PartialEq, Zeroize, ZeroizeOnDrop)]
pub struct EncryptedShareKey(pub(crate) Vec<u8>);

impl EncryptedShareKey {
    pub fn value(self) -> Vec<u8> {
        self.0.clone()
    }
}

impl AsRef<[u8]> for EncryptedShareKey {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq, ZeroizeOnDrop)]
pub struct ShareKey {
    pub key_rotation: u8,
    pub key: EncryptedShareKey,
}

impl ShareKey {
    pub fn new(key_rotation: u8, key: EncryptedShareKey) -> Self {
        Self { key_rotation, key }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ShareKeys {
    pub keys: Vec<ShareKey>,
}

impl ShareKeys {
    pub fn new(keys: Vec<ShareKey>) -> Self {
        Self { keys }
    }

    pub fn latest(&self) -> Option<&ShareKey> {
        self.keys.iter().max_by_key(|k| k.key_rotation)
    }

    pub fn latest_or_err(&self) -> anyhow::Result<&ShareKey> {
        match self.latest() {
            Some(k) => Ok(k),
            None => anyhow::bail!("No latest ShareKey"),
        }
    }
}
