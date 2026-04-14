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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AddressId(pub(crate) String);
display_for_basic!(AddressId);

impl AddressId {
    pub fn new(id: String) -> Self {
        Self(id)
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug)]
pub struct Address {
    pub id: AddressId,
    pub email: String,
    pub keys: Vec<AddressKey>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct AddressKeyId(pub(crate) String);
display_for_basic!(AddressKeyId);

impl AddressKeyId {
    pub fn new(id: String) -> Self {
        Self(id)
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug)]
pub struct AddressKey {
    pub id: AddressKeyId,
    pub primary: bool,
    pub active: bool,
    pub private_key: String,
    pub token: Option<String>,
    pub signature: Option<String>,
}
