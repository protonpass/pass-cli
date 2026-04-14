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

#[derive(Debug)]
pub enum PgpCryptoError {
    Unknown,
}

impl std::fmt::Display for PgpCryptoError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for PgpCryptoError {}

#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct PrivateKey {
    content: Vec<u8>,
}

impl PrivateKey {
    pub fn new(content: Vec<u8>) -> Self {
        Self { content }
    }
}

impl AsRef<[u8]> for PrivateKey {
    fn as_ref(&self) -> &[u8] {
        &self.content
    }
}

#[derive(Clone)]
pub struct PublicKey {
    content: Vec<u8>,
}

impl PublicKey {
    pub fn new(content: Vec<u8>) -> Self {
        Self { content }
    }
}

impl AsRef<[u8]> for PublicKey {
    fn as_ref(&self) -> &[u8] {
        &self.content
    }
}

#[derive(Zeroize, ZeroizeOnDrop)]
pub struct PlainText(pub(crate) Vec<u8>);

impl PlainText {
    pub fn new(content: Vec<u8>) -> Self {
        Self(content)
    }
}

impl AsRef<[u8]> for PlainText {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

pub enum Signature {
    Bytes(Vec<u8>),
    Armored(String),
}

pub enum DataToDecrypt {
    RawData(Vec<u8>),
    DataWithSignature { data: Vec<u8>, signature: Signature },
}
