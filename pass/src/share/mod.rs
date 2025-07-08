mod keys;
mod list;
mod open_key;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EncryptedShareKey(pub(crate) Vec<u8>);

#[derive(Clone, Debug, Eq, PartialEq)]
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

    pub fn find_by_rotation(&self, key_rotation: u8) -> Option<&ShareKey> {
        self.keys.iter().find(|k| k.key_rotation == key_rotation)
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
