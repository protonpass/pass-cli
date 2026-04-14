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

use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Clone, serde::Deserialize, serde::Serialize, Zeroize, ZeroizeOnDrop)]
pub struct DecryptedFolderKey {
    pub key_rotation: u8,
    pub(crate) key: Vec<u8>,
}

impl DecryptedFolderKey {
    pub fn new(key_rotation: u8, key: Vec<u8>) -> Self {
        Self { key_rotation, key }
    }

    pub fn value(&self) -> Vec<u8> {
        self.key.clone()
    }

    pub fn key(&self) -> &[u8] {
        &self.key
    }
}

impl AsRef<[u8]> for DecryptedFolderKey {
    fn as_ref(&self) -> &[u8] {
        &self.key
    }
}
