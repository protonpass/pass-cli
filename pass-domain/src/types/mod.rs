use crate::AddressKeyId;
pub use key::*;
pub use share_key::*;
use std::collections::{BTreeMap, HashMap};
pub use user_key::*;
use zeroize::{Zeroize, ZeroizeOnDrop};

mod key;
mod share_key;
mod user_key;

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
