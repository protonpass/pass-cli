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

use std::collections::HashMap;
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Clone, Debug, Zeroize, ZeroizeOnDrop)]
pub struct Passphrase(pub(crate) Vec<u8>);

impl Passphrase {
    pub fn new(value: Vec<u8>) -> Self {
        Self(value)
    }
}

impl AsRef<[u8]> for Passphrase {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Clone, Debug, ZeroizeOnDrop)]
pub struct KeyPassphrase {
    pub id: String,
    pub passphrase: Passphrase,
}

#[derive(Clone, Debug)]
pub struct KeyPassphrases {
    passphrases: Vec<KeyPassphrase>,
}

impl KeyPassphrases {
    pub fn new(passphrases: Vec<KeyPassphrase>) -> KeyPassphrases {
        Self { passphrases }
    }

    pub fn into_map(self) -> HashMap<String, Passphrase> {
        let mut res = HashMap::new();
        for passphrase in self.passphrases {
            res.insert(passphrase.id.to_string(), passphrase.passphrase.clone());
        }
        res
    }
}
