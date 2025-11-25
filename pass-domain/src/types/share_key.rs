use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Clone, serde::Deserialize, serde::Serialize, Zeroize, ZeroizeOnDrop)]
pub struct DecryptedShareKey {
    pub key_rotation: u8,
    pub(crate) key: Vec<u8>,
}

impl DecryptedShareKey {
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

impl AsRef<[u8]> for DecryptedShareKey {
    fn as_ref(&self) -> &[u8] {
        &self.key
    }
}
