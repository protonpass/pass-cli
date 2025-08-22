use crate::Passphrase;
use anyhow::Result;
use zeroize::{Zeroize, ZeroizeOnDrop};

mod constants;
pub(crate) mod encrypt_invite_keys;
pub(crate) mod open_invite_key;
pub(crate) mod reencrypt_group_invite_keys;
pub(crate) mod reencrypt_invite_keys;
pub(crate) mod share_key;

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
    pub content: Vec<u8>,
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
    pub content: Vec<u8>,
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

#[async_trait::async_trait]
pub trait PgpCrypto {
    async fn encrypt(&self, data: Vec<u8>, key: PublicKey) -> Result<Vec<u8>>;
    async fn encrypt_and_sign(
        &self,
        data: PlainText,
        encryption_key: PublicKey,
        signing_key: PrivateKey,
        signing_context: Option<String>,
    ) -> Result<Vec<u8>>;

    async fn sign(&self, data: Vec<u8>, signing_key: PrivateKey) -> Result<Vec<u8>>;

    async fn decrypt(&self, data: Vec<u8>, keys: Vec<PrivateKey>) -> Result<Vec<u8>>;
    async fn decrypt_and_verify(
        &self,
        data: Vec<u8>,
        decryption_keys: Vec<PrivateKey>,
        verification_keys: Vec<PublicKey>,
        verification_context: Option<String>,
    ) -> Result<Vec<u8>>;
    async fn decrypt_and_verify_data(
        &self,
        data: DataToDecrypt,
        decryption_keys: Vec<PrivateKey>,
        verification_keys: Vec<PublicKey>,
        verification_context: Option<String>,
    ) -> Result<Vec<u8>>;
    async fn unarmor(&self, armored: String) -> Result<Vec<u8>>;

    async fn open_private_key(&self, key: PrivateKey, passphrase: Passphrase)
    -> Result<PrivateKey>;
    async fn get_public_key(&self, key: PrivateKey) -> Result<PublicKey>;
}
