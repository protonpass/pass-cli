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
