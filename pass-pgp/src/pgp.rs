use anyhow::{Context, anyhow};
use pass_domain::{
    DataToArmor, DataToDecrypt, Passphrase, PlainText, PrivateKey, PublicKey, Signature,
};
use proton_crypto::crypto::{
    ArmorerSync, DataEncoding, Decryptor, DecryptorSync, DetachedSignatureVariant, Encryptor,
    EncryptorSync, KeyGenerator, KeyGeneratorAlgorithm, KeyGeneratorSync, PGPProvider,
    PGPProviderSync, Signer, SignerSync, UnixTimestamp, VerifiedData,
};

pub struct NativePgpCrypto;

#[async_trait::async_trait]
impl pass_domain::PgpCrypto for NativePgpCrypto {
    async fn encrypt(&self, data: Vec<u8>, key: PublicKey) -> anyhow::Result<Vec<u8>> {
        let provider = proton_crypto::new_pgp_provider();

        let public_key = provider
            .public_key_import(key.as_ref(), DataEncoding::Bytes)
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

    async fn encrypt_and_sign(
        &self,
        data: PlainText,
        encryption_key: PublicKey,
        signing_key: PrivateKey,
        signing_context: Option<String>,
    ) -> anyhow::Result<Vec<u8>> {
        let provider = proton_crypto::new_pgp_provider();

        let public_key = provider
            .public_key_import(encryption_key.as_ref(), DataEncoding::Bytes)
            .context("Error importing public key")?;
        let private_key = provider
            .private_key_import_unlocked(signing_key.as_ref(), DataEncoding::Bytes)
            .context("Could not import key")?;
        let encryptor = provider
            .new_encryptor()
            .with_encryption_key(&public_key)
            .with_signing_key(&private_key);

        let signing_context =
            signing_context.map(|context| provider.new_signing_context(context, true));

        let encryptor = match signing_context {
            Some(ref ctx) => encryptor.with_signing_context(ctx),
            None => encryptor,
        };

        let res = encryptor
            .encrypt(data)
            .context("Could not encrypt and sign data")?
            .as_ref()
            .to_vec();

        Ok(res)
    }

    async fn sign(&self, data: Vec<u8>, signing_key: PrivateKey) -> anyhow::Result<Vec<u8>> {
        let provider = proton_crypto::new_pgp_provider();
        let private_key = provider
            .private_key_import_unlocked(signing_key.as_ref(), DataEncoding::Bytes)
            .context("Could not import key")?;
        let res = provider
            .new_signer()
            .with_signing_key(&private_key)
            .sign_detached(data, DataEncoding::Bytes)
            .context("Could not sign data")?;

        Ok(res)
    }

    async fn decrypt(&self, data: Vec<u8>, keys: Vec<PrivateKey>) -> anyhow::Result<Vec<u8>> {
        let provider = proton_crypto::new_pgp_provider();

        let mut private_keys = vec![];

        for key in keys {
            let private_key = provider
                .private_key_import_unlocked(key.as_ref(), DataEncoding::Bytes)
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
        verification_context: Option<String>,
    ) -> anyhow::Result<Vec<u8>> {
        let provider = proton_crypto::new_pgp_provider();

        let mut private_keys = vec![];

        for key in decryption_keys {
            let private_key = provider
                .private_key_import_unlocked(key.as_ref(), DataEncoding::Bytes)
                .context("Error importing private key")?;
            private_keys.push(private_key);
        }

        let mut public_keys = vec![];
        for key in verification_keys {
            let as_public_key = provider
                .public_key_import(key.as_ref(), DataEncoding::Bytes)
                .context("Could not import key")?;
            public_keys.push(as_public_key);
        }

        let decryptor = provider
            .new_decryptor()
            .with_decryption_keys(&private_keys)
            .with_verification_keys(&public_keys);

        let signing_context = verification_context
            .map(|context| provider.new_verification_context(context, true, UnixTimestamp::zero()));

        let decryptor = match signing_context {
            Some(ref ctx) => decryptor.with_verification_context(ctx),
            None => decryptor,
        };

        let res = decryptor
            .decrypt(data, DataEncoding::Bytes)
            .context("Could not decrypt data")?;

        match res.verification_result() {
            Ok(info) => trace!("Verification successful: {info:?}"),
            Err(err) => {
                warn!("Error verifying signature: {err:?}");
                return Err(anyhow!("Error verifying signature"));
            }
        }

        Ok(res.as_ref().to_vec())
    }

    async fn decrypt_and_verify_data(
        &self,
        data: DataToDecrypt,
        decryption_keys: Vec<PrivateKey>,
        verification_keys: Vec<PublicKey>,
        verification_context: Option<String>,
    ) -> anyhow::Result<Vec<u8>> {
        let provider = proton_crypto::new_pgp_provider();

        let mut private_keys = vec![];

        for key in decryption_keys {
            let private_key = provider
                .private_key_import_unlocked(key.as_ref(), DataEncoding::Bytes)
                .context("Error importing private key")?;
            private_keys.push(private_key);
        }

        let mut public_keys = vec![];
        for key in verification_keys {
            public_keys.push(
                provider
                    .public_key_import(key.as_ref(), DataEncoding::Bytes)
                    .context("Could not import key")?,
            );
        }

        let decryptor = provider
            .new_decryptor()
            .with_decryption_keys(&private_keys)
            .with_verification_keys(&public_keys);

        let signing_context = verification_context
            .map(|context| provider.new_verification_context(context, true, UnixTimestamp::zero()));

        let decryptor = match signing_context {
            Some(ref ctx) => decryptor.with_verification_context(ctx),
            None => decryptor,
        };

        let res = match data {
            DataToDecrypt::RawData(data) => decryptor
                .decrypt(data, DataEncoding::Auto)
                .context("Could not decrypt data")?
                .as_ref()
                .to_vec(),
            DataToDecrypt::DataWithSignature { data, signature } => match signature {
                Signature::Bytes(sig) => decryptor
                    .with_detached_signature(sig, DetachedSignatureVariant::Plaintext, false)
                    .decrypt(data, DataEncoding::Auto)
                    .context("Could not decrypt data")?
                    .as_ref()
                    .to_vec(),
                Signature::Armored(sig) => decryptor
                    .with_detached_signature(
                        sig.into_bytes(),
                        DetachedSignatureVariant::Plaintext,
                        true,
                    )
                    .decrypt(data, DataEncoding::Auto)
                    .context("Could not decrypt data")?
                    .as_ref()
                    .to_vec(),
            },
        };

        Ok(res)
    }

    async fn armor(&self, data: DataToArmor) -> anyhow::Result<String> {
        let provider = proton_crypto::new_pgp_provider();
        let armorer = provider.armorer();

        let armored = match data {
            DataToArmor::Message(content) => armorer.armor_message(&content),
            DataToArmor::Signature(content) => armorer.armor_signature(&content),
            DataToArmor::PrivateKey(content) => armorer.armor_private_key(&content),
            DataToArmor::PublicKey(content) => armorer.armor_public_key(&content),
        }
        .map_err(|e| anyhow!("Error armoring data: {e:?}"))?;

        match String::from_utf8(armored) {
            Ok(content) => Ok(content),
            Err(e) => Err(anyhow!("Error armoring data: {e:?}")),
        }
    }

    async fn unarmor(&self, armored: String) -> anyhow::Result<Vec<u8>> {
        let provider = proton_crypto::new_pgp_provider();
        match provider.armorer().unarmor(armored) {
            Ok(data) => Ok(data),
            Err(e) => Err(anyhow!("Error unarmoring data: {}", e)),
        }
    }

    async fn open_private_key(
        &self,
        key: PrivateKey,
        passphrase: Passphrase,
    ) -> anyhow::Result<PrivateKey> {
        let provider = proton_crypto::new_pgp_provider();
        let imported = provider
            .private_key_import(key.as_ref(), passphrase.as_ref(), DataEncoding::Auto)
            .context("Could not import private key")?;

        let exported = provider.private_key_export_unlocked(&imported, DataEncoding::Bytes)?;
        Ok(PrivateKey::new(exported.as_ref().to_vec()))
    }

    async fn get_public_key(&self, key: PrivateKey) -> anyhow::Result<PublicKey> {
        let provider = proton_crypto::new_pgp_provider();
        let imported = provider
            .private_key_import_unlocked(key.as_ref(), DataEncoding::Auto)
            .context("Could not import private key")?;

        let public = provider
            .private_key_to_public_key(&imported)
            .context("Could not get public key from private key")?;
        let exported = provider
            .public_key_export(&public, DataEncoding::Bytes)
            .context("Could not export public key")?;

        Ok(PublicKey::new(exported.as_ref().to_vec()))
    }

    async fn generate_key_pair(
        &self,
        name: String,
        email: String,
    ) -> anyhow::Result<(PrivateKey, PublicKey)> {
        let provider = proton_crypto::new_pgp_provider();
        let generator = provider.new_key_generator();

        let now = jiff::Timestamp::now().as_second();
        let key = generator
            .with_generation_time(UnixTimestamp(now as u64))
            .with_algorithm(KeyGeneratorAlgorithm::ECC)
            .with_user_id(&name, &email)
            .generate()
            .map_err(|e| anyhow!("Error generating key pair: {}", e))?;

        let private = provider
            .private_key_export_unlocked(&key, DataEncoding::Bytes)
            .map_err(|e| anyhow!("Error exporting private key: {}", e))?;

        let public = provider
            .private_key_to_public_key(&key)
            .map_err(|e| anyhow!("Error obtaining public key: {}", e))?;

        let public_exported = provider
            .public_key_export(&public, DataEncoding::Bytes)
            .map_err(|e| anyhow!("Error exporting public key: {}", e))?;

        Ok((
            PrivateKey::new(private.as_ref().to_vec()),
            PublicKey::new(public_exported.as_ref().to_vec()),
        ))
    }
}
