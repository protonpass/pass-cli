use anyhow::{Context, Result, anyhow};
use muon::rest::core::v4::keys::Key;
use pass::{
    ApiKey, ApiKeySalt, Passphrase, PrivateKey, UnlockedAddressKey, UnlockedAddressKeys, UserKey,
};
use pass_domain::AddressKey;
use proton_crypto::crypto::{
    DataEncoding, Decryptor, DecryptorSync, PGPProviderSync, Verifier, VerifierSync,
};
use proton_crypto_account::keys::{
    ArmoredPrivateKey, EncryptedKeyToken, KeyId, KeyTokenSignature, LockedKey, UserKeys,
};
use proton_crypto_account::salts::{KeySalt, KeySecret, Salt, Salts};
use std::collections::HashMap;

pub struct AccountCrypto;

impl AccountCrypto {
    pub fn generate_passphrases(
        &self,
        key_salts: Vec<ApiKeySalt>,
        pass: &str,
    ) -> Result<HashMap<String, Passphrase>> {
        let srp_provider = proton_crypto::new_srp_provider();

        let salts: Vec<Salt> = key_salts.iter().map(salt_to_salt).collect();
        let salts = Salts::new(salts);

        let mut res = HashMap::new();
        for salt in key_salts {
            if salt.key_salt.is_some() {
                let key_secret = salts
                    .salt_for_key(&srp_provider, &KeyId(salt.id.clone()), pass.as_bytes())
                    .context("Failed to get salt for key")?;

                let passphrase = Passphrase::new(key_secret.as_bytes().to_vec());
                res.insert(salt.id, passphrase);
            }
        }

        Ok(res)
    }

    pub fn open_user_keys(
        &self,
        keys: Vec<ApiKey>,
        passphrases: HashMap<String, Passphrase>,
    ) -> Result<Vec<UserKey>> {
        let locked_user_keys = keys.into_iter().map(key_to_locked_key).collect();
        let user_keys = UserKeys(locked_user_keys);

        let provider = proton_crypto::new_pgp_provider();

        let mut keys = Vec::new();

        for user_key in user_keys.0.iter() {
            let key_id = &user_key.id.0;
            let key_secret = match passphrases.get(key_id) {
                Some(key_secret) => KeySecret::new(key_secret.as_ref().to_vec()),
                None => return Err(anyhow!("Could not find passphrase for key {key_id}")),
            };

            let res = user_keys.unlock(&provider, &key_secret);
            debug!(
                "User key unlock: Success: {} | Failed: {}",
                res.unlocked_keys.len(),
                res.failed.len()
            );

            for key in res.unlocked_keys {
                let exported_public = provider
                    .public_key_export(&key.public_key, DataEncoding::Bytes)
                    .context("Failed to export public key")?
                    .as_ref()
                    .to_vec();
                let exported_private = provider
                    .private_key_export_unlocked(&key.private_key, DataEncoding::Bytes)
                    .context("Failed to export private key")?;
                keys.push(UserKey {
                    public_key: exported_public,
                    private_key: exported_private.as_ref().to_vec(),
                })
            }
        }

        Ok(keys)
    }

    pub fn open_address_keys(
        &self,
        user_keys: Vec<UserKey>,
        address_keys: Vec<AddressKey>,
    ) -> Result<UnlockedAddressKeys> {
        let mut unlocked_keys = Vec::with_capacity(address_keys.len());
        for address_key in address_keys {
            let unlocked = self
                .unlock_address_key(&user_keys, address_key)
                .context("Error unlocking address key")?;
            unlocked_keys.push(unlocked);
        }

        Ok(UnlockedAddressKeys::new(unlocked_keys))
    }

    fn unlock_address_key(
        &self,
        user_keys: &[UserKey],
        address_key: AddressKey,
    ) -> Result<UnlockedAddressKey> {
        match (&address_key.signature, &address_key.token) {
            (Some(signature), Some(token)) => self.unlock_address_key_with_detached_signature(
                &address_key,
                user_keys,
                signature,
                token,
            ),
            (None, Some(token)) => {
                self.unlock_address_key_with_embedded_signature(&address_key, user_keys, token)
            }
            _ => Err(anyhow::anyhow!("Unsupported address key")),
        }
    }

    fn unlock_address_key_with_embedded_signature(
        &self,
        address_key: &AddressKey,
        user_keys: &[UserKey],
        token: &str,
    ) -> Result<UnlockedAddressKey> {
        let provider = proton_crypto::new_pgp_provider();

        let unlock_with_key = |key: &UserKey| -> Result<UnlockedAddressKey> {
            let decryptor = provider.new_decryptor();
            let uk_as_private_key = provider
                .private_key_import_unlocked(&key.private_key, DataEncoding::Bytes)
                .context("Error importing private user key")?;
            let uk_as_public_key = provider
                .public_key_import(&key.public_key, DataEncoding::Bytes)
                .context("Error importing public user key")?;

            let decrypted_token = decryptor
                .with_decryption_key(&uk_as_private_key)
                .with_verification_key(&uk_as_public_key)
                .decrypt(token, DataEncoding::Armor)
                .context("Error decrypting and verifying address key token")?;

            let instance = self.unlock_with_passphrase(address_key, decrypted_token.as_ref())?;
            Ok(instance)
        };

        for key in user_keys {
            match unlock_with_key(key) {
                Ok(instance) => {
                    return Ok(instance);
                }
                Err(e) => {
                    warn!("Error unlocking key with embedded signature: {}", e);
                    continue;
                }
            }
        }

        Err(anyhow!("No user key could unlock address key"))
    }

    fn unlock_address_key_with_detached_signature(
        &self,
        locked_key: &AddressKey,
        user_keys: &[UserKey],
        signature: &str,
        token: &str,
    ) -> Result<UnlockedAddressKey> {
        let provider = proton_crypto::new_pgp_provider();

        let unlock_with_key = |key: &UserKey| -> Result<UnlockedAddressKey> {
            let verifier = provider.new_verifier();
            let decryptor = provider.new_decryptor();
            let as_private_key = provider
                .private_key_import_unlocked(&key.private_key, DataEncoding::Bytes)
                .context("Error importing private user key")?;
            let decrypted_token = decryptor
                .with_decryption_key(&as_private_key)
                .decrypt(token, DataEncoding::Armor)
                .context("Error decrypting address token")?;

            let as_public_key = provider.public_key_import(&key.public_key, DataEncoding::Bytes)?;

            let verified = verifier
                .with_verification_key(&as_public_key)
                .verify_detached(&decrypted_token, signature, DataEncoding::Armor);

            match verified {
                Ok(_) => self.unlock_with_passphrase(locked_key, decrypted_token.as_ref()),
                Err(e) => Err(anyhow::anyhow!("Signature does not match: {}", e)),
            }
        };

        for key in user_keys {
            match unlock_with_key(key) {
                Ok(instance) => {
                    return Ok(instance);
                }
                Err(e) => {
                    warn!("Error unlocking key with detached signature: {}", e);
                    continue;
                }
            }
        }

        Err(anyhow!("No user key could unlock address key"))
    }

    fn unlock_with_passphrase(
        &self,
        locked_key: &AddressKey,
        passphrase: &[u8],
    ) -> Result<UnlockedAddressKey> {
        let provider = proton_crypto::new_pgp_provider();

        let as_private_key = provider
            .private_key_import(&locked_key.private_key, passphrase, DataEncoding::Armor)
            .context("Error importing private address key")?;

        let exported = provider
            .private_key_export_unlocked(&as_private_key, DataEncoding::Bytes)
            .context("Error exporting private key")?;

        Ok(UnlockedAddressKey {
            id: locked_key.id.clone(),
            private_key: PrivateKey {
                content: exported.as_ref().to_vec(),
            },
        })
    }
}

fn key_to_locked_key(key: Key) -> LockedKey {
    LockedKey {
        id: KeyId(key.id),
        version: 3,
        private_key: ArmoredPrivateKey(key.private_key),
        token: key.token.map(EncryptedKeyToken),
        signature: key.signature.map(KeyTokenSignature),
        activation: None,
        primary: key.primary.into(),
        active: key.active.into(),
        flags: None,
        recovery_secret: None,
        recovery_secret_signature: None,
        address_forwarding_id: None,
    }
}

fn salt_to_salt(salt: &ApiKeySalt) -> Salt {
    Salt {
        id: KeyId(salt.id.clone()),
        key_salt: salt.key_salt.as_ref().map(|s| KeySalt(s.to_string())),
    }
}
