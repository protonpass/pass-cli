use anyhow::Result;

pub(crate) mod open_invite_key;
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

#[derive(Clone)]
pub struct PrivateKey {
    pub content: Vec<u8>,
}

impl Drop for PrivateKey {
    fn drop(&mut self) {
        for i in 0..self.content.len() {
            self.content[i] = 0;
        }
    }
}

#[derive(Clone)]
pub struct PublicKey {
    pub content: Vec<u8>,
}

impl Drop for PublicKey {
    fn drop(&mut self) {
        for i in 0..self.content.len() {
            self.content[i] = 0;
        }
    }
}

#[async_trait::async_trait]
pub trait PgpCrypto {
    async fn encrypt(&self, data: Vec<u8>, key: PublicKey) -> Result<Vec<u8>>;
    async fn encrypt_and_sign(
        &self,
        data: Vec<u8>,
        encryption_key: PublicKey,
        signing_key: PrivateKey,
    ) -> Result<Vec<u8>>;

    async fn sign(&self, data: Vec<u8>, signing_key: PrivateKey) -> Result<Vec<u8>>;

    async fn decrypt(&self, data: Vec<u8>, keys: Vec<PrivateKey>) -> Result<Vec<u8>>;
    async fn decrypt_and_verify(
        &self,
        data: Vec<u8>,
        decryption_keys: Vec<PrivateKey>,
        verification_keys: Vec<PublicKey>,
    ) -> Result<Vec<u8>>;
    async fn unarmor(&self, armored: String) -> Result<Vec<u8>>;
}
