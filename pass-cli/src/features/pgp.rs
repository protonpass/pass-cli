use anyhow::{Context, anyhow};
use pass::{PrivateKey, PublicKey};
use proton_crypto::crypto::{
    ArmorerSync, DataEncoding, Decryptor, DecryptorSync, Encryptor, EncryptorSync, PGPProviderSync,
};

pub struct NativePgpCrypto;

#[async_trait::async_trait]
impl pass::PgpCrypto for NativePgpCrypto {
    async fn encrypt(&self, data: Vec<u8>, key: PublicKey) -> anyhow::Result<Vec<u8>> {
        let provider = proton_crypto::new_pgp_provider();

        let public_key = provider
            .public_key_import(&key.content, DataEncoding::Bytes)
            .context("Error importing public key")?;
        let res = provider
            .new_encryptor()
            .with_encryption_key(&public_key)
            .encrypt(data)
            .context("Could not encrypt data")?
            .as_ref()
            .to_vec();

        Ok(res)
    }

    async fn sign(&self, data: Vec<u8>, signing_key: PrivateKey) -> anyhow::Result<Vec<u8>> {
        let provider = proton_crypto::new_pgp_provider();
        let private_key = provider
            .private_key_import_unlocked(&signing_key.content, DataEncoding::Bytes)
            .context("Could not import key")?;
        let res = provider
            .new_encryptor()
            .with_signing_key(&private_key)
            .encrypt(data)
            .context("Could not sign data")?
            .as_ref()
            .to_vec();

        Ok(res)
    }

    async fn encrypt_and_sign(
        &self,
        data: Vec<u8>,
        encryption_key: PublicKey,
        signing_key: PrivateKey,
    ) -> anyhow::Result<Vec<u8>> {
        let provider = proton_crypto::new_pgp_provider();

        let public_key = provider
            .public_key_import(&encryption_key.content, DataEncoding::Bytes)
            .context("Error importing public key")?;
        let private_key = provider
            .private_key_import_unlocked(&signing_key.content, DataEncoding::Bytes)
            .context("Could not import key")?;
        let res = provider
            .new_encryptor()
            .with_encryption_key(&public_key)
            .with_signing_key(&private_key)
            .encrypt(data)
            .context("Could not encrypt and sign data")?
            .as_ref()
            .to_vec();

        Ok(res)
    }

    async fn decrypt(&self, data: Vec<u8>, keys: Vec<PrivateKey>) -> anyhow::Result<Vec<u8>> {
        let provider = proton_crypto::new_pgp_provider();

        let mut private_keys = vec![];

        for key in keys {
            let private_key = provider
                .private_key_import_unlocked(&key.content, DataEncoding::Bytes)
                .context("Error importing private key")?;
            private_keys.push(private_key);
        }

        let res = provider
            .new_decryptor()
            .with_decryption_keys(&private_keys)
            .decrypt(data, DataEncoding::Bytes)
            .context("Could not decrypt data")?
            .as_ref()
            .to_vec();

        Ok(res)
    }

    async fn decrypt_and_verify(
        &self,
        data: Vec<u8>,
        decryption_keys: Vec<PrivateKey>,
        verification_keys: Vec<PublicKey>,
    ) -> anyhow::Result<Vec<u8>> {
        let provider = proton_crypto::new_pgp_provider();

        let mut private_keys = vec![];

        for key in decryption_keys {
            let private_key = provider
                .private_key_import_unlocked(&key.content, DataEncoding::Bytes)
                .context("Error importing private key")?;
            private_keys.push(private_key);
        }

        let mut public_keys = vec![];
        for key in verification_keys {
            public_keys.push(
                provider
                    .public_key_import(&key.content, DataEncoding::Bytes)
                    .context("Could not import key")?,
            );
        }

        let res = provider
            .new_decryptor()
            .with_decryption_keys(&private_keys)
            .with_verification_keys(&public_keys)
            .decrypt(data, DataEncoding::Bytes)
            .context("Could not decrypt data")?
            .as_ref()
            .to_vec();

        Ok(res)
    }

    async fn unarmor(&self, armored: String) -> anyhow::Result<Vec<u8>> {
        let provider = proton_crypto::new_pgp_provider();
        match provider.armorer().unarmor(armored) {
            Ok(data) => Ok(data),
            Err(e) => Err(anyhow!("Error unarmoring data: {}", e)),
        }
    }
}
