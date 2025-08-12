use anyhow::Result;
use zeroize::{Zeroize, ZeroizeOnDrop};

mod constants;
pub(crate) mod encrypt_invite_keys;
pub(crate) mod open_invite_key;
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

#[derive(Clone)]
pub struct PublicKey {
    pub content: Vec<u8>,
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
    async fn unarmor(&self, armored: String) -> Result<Vec<u8>>;
}
