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

use super::key::*;
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Clone, Debug)]
pub struct KeySalt {
    pub id: String,
    pub key_salt: Option<String>,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct LockedUserKey {
    pub id: String,
    pub private_key: String,
    pub token: Option<String>,
    pub signature: Option<String>,
    pub primary: bool,
    pub active: bool,
}

#[derive(Clone, serde::Deserialize, serde::Serialize, Zeroize, ZeroizeOnDrop)]
pub struct UserKey {
    pub public_key: Vec<u8>,
    pub private_key: Vec<u8>,
}

impl UserKey {
    pub fn into_keys(self) -> (PrivateKey, PublicKey) {
        (
            PrivateKey::new(self.private_key.clone()),
            PublicKey::new(self.public_key.clone()),
        )
    }
}

pub trait UserKeyExt {
    fn split_keys(self) -> (Vec<PrivateKey>, Vec<PublicKey>);
}

impl UserKeyExt for Vec<UserKey> {
    fn split_keys(self) -> (Vec<PrivateKey>, Vec<PublicKey>) {
        let mut private = Vec::with_capacity(self.len());
        let mut public = Vec::with_capacity(self.len());

        for key in self {
            let (pr, pu) = key.into_keys();
            private.push(pr);
            public.push(pu);
        }

        (private, public)
    }
}
