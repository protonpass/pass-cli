/*
 *  Copyright (c) 2026 Proton AG
 *  This file is part of Proton AG and Proton Pass.
 *
 *  Proton Pass is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  Proton Pass is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with Proton Pass.  If not, see <https://www.gnu.org/licenses/>.
 *
 */

use anyhow::{Context, Result, anyhow};
use pass_domain::{
    AccountCrypto, AddressKey, KeySalt as DomainKeySalt, LockedUserKey, Passphrase, PrivateKey,
    PublicKey, UnlockedAddressKey, UnlockedAddressKeys, UserKey, UserKeyExt,
};
use proton_crypto::crypto::{
    DataEncoding, Decryptor, DecryptorSync, PGPProviderSync, Verifier, VerifierSync,
};
use proton_crypto_account::keys::{
    ArmoredPrivateKey, EncryptedKeyToken, KeyId, KeyTokenSignature, LockedKey, UserKeys,
};
use proton_crypto_account::salts::{KeySalt, KeySecret, Salt, Salts};
use std::collections::HashMap;

pub struct ProtonAccountCrypto;

impl ProtonAccountCrypto {
    fn unlock_address_key(
        &self,
        private_keys: &[PrivateKey],
        public_keys: &[PublicKey],
        address_key: AddressKey,
    ) -> Result<UnlockedAddressKey> {
        match (&address_key.signature, &address_key.token) {
            (Some(signature), Some(token)) => self.unlock_address_key_with_detached_signature(
                &address_key,
                private_keys,
                public_keys,
                signature,
                token,
            ),
            (None, Some(token)) => self.unlock_address_key_with_embedded_signature(
                &address_key,
                private_keys,
                public_keys,
                token,
            ),
            _ => Err(anyhow::anyhow!("Unsupported address key")),
        }
    }

    fn unlock_address_key_with_embedded_signature(
        &self,
        address_key: &AddressKey,
        private_keys: &[PrivateKey],
        public_keys: &[PublicKey],
        token: &str,
    ) -> Result<UnlockedAddressKey> {
        let provider = proton_crypto::new_pgp_provider();
        let mut imported_public_keys = Vec::with_capacity(public_keys.len());
        for public_key in public_keys {
            let as_public_key = provider
                .public_key_import(public_key.as_ref(), DataEncoding::Bytes)
                .context("Error importing public key")?;
            imported_public_keys.push(as_public_key);
        }

        for key in private_keys {
            match unlock_address_key_with_key(
                &provider,
                key,
                &imported_public_keys,
                address_key,
                token,
            ) {
                Ok(instance) => {
                    return Ok(instance);
                }
                Err(e) => {
                    warn!("Error unlocking key with embedded signature: {:#}", e);
                    continue;
                }
            }
        }

        Err(anyhow!(
            "No user key could unlock address key with embedded signature"
        ))
    }

    fn unlock_address_key_with_detached_signature(
        &self,
        locked_key: &AddressKey,
        private_keys: &[PrivateKey],
        public_keys: &[PublicKey],
        signature: &str,
        token: &str,
    ) -> Result<UnlockedAddressKey> {
        let provider = proton_crypto::new_pgp_provider();
        let mut imported_public_keys = Vec::with_capacity(public_keys.len());
        for public_key in public_keys {
            let as_public_key = provider
                .public_key_import(public_key.as_ref(), DataEncoding::Bytes)
                .context("Error importing public key")?;
            imported_public_keys.push(as_public_key);
        }

        let unlock_with_key = |key: &PrivateKey| -> Result<UnlockedAddressKey> {
            let verifier = provider.new_verifier();
            let decryptor = provider.new_decryptor();
            let as_private_key = provider
                .private_key_import_unlocked(key.as_ref(), DataEncoding::Auto)
                .context("Error importing private key")?;
            let decrypted_token = decryptor
                .with_decryption_key(&as_private_key)
                .decrypt(token, DataEncoding::Armor)
                .context("Error decrypting address token")?;

            let verified = verifier
                .with_verification_keys(&imported_public_keys)
                .verify_detached(&decrypted_token, signature, DataEncoding::Armor);

            match verified {
                Ok(_) => self.unlock_with_passphrase(locked_key, decrypted_token.as_ref()),
                Err(e) => Err(anyhow::anyhow!("Signature does not match: {}", e)),
            }
        };

        for key in private_keys {
            match unlock_with_key(key) {
                Ok(instance) => {
                    return Ok(instance);
                }
                Err(e) => {
                    warn!("Error unlocking key with detached signature: {:#}", e);
                    continue;
                }
            }
        }

        Err(anyhow!(
            "No user key could unlock address key with detached signature"
        ))
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
            private_key: PrivateKey::new(exported.as_ref().to_vec()),
        })
    }

    fn perform_open_address_keys(
        &self,
        private_keys: Vec<PrivateKey>,
        public_keys: Vec<PublicKey>,
        address_keys: Vec<AddressKey>,
    ) -> Result<UnlockedAddressKeys> {
        let mut unlocked_keys = Vec::with_capacity(address_keys.len());
        for address_key in address_keys {
            let unlocked = self
                .unlock_address_key(&private_keys, &public_keys, address_key)
                .context("Error unlocking address key")?;
            unlocked_keys.push(unlocked);
        }

        Ok(UnlockedAddressKeys::new(unlocked_keys))
    }
}

#[async_trait::async_trait]
impl AccountCrypto for ProtonAccountCrypto {
    async fn generate_passphrases(
        &self,
        key_salts: Vec<DomainKeySalt>,
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

    async fn open_user_keys(
        &self,
        keys: Vec<LockedUserKey>,
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
                None => {
                    warn!("Could not find passphrase for key {key_id}");
                    continue;
                }
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

    async fn open_address_keys(
        &self,
        user_keys: Vec<UserKey>,
        address_keys: Vec<AddressKey>,
    ) -> Result<UnlockedAddressKeys> {
        let (private, public) = user_keys.split_keys();
        self.open_address_keys_with_keys(private, public, address_keys)
            .await
    }

    async fn open_address_keys_with_keys(
        &self,
        private_keys: Vec<PrivateKey>,
        public_keys: Vec<PublicKey>,
        address_keys: Vec<AddressKey>,
    ) -> Result<UnlockedAddressKeys> {
        self.perform_open_address_keys(private_keys, public_keys, address_keys)
    }
}

fn key_to_locked_key(key: LockedUserKey) -> LockedKey {
    LockedKey {
        id: KeyId(key.id),
        version: 3,
        private_key: ArmoredPrivateKey(key.private_key),
        token: key.token.map(EncryptedKeyToken),
        signature: key.signature.map(KeyTokenSignature),
        activation: None,
        primary: key.primary,
        active: key.active,
        flags: None,
        recovery_secret: None,
        recovery_secret_signature: None,
        address_forwarding_id: None,
    }
}

fn salt_to_salt(salt: &DomainKeySalt) -> Salt {
    Salt {
        id: KeyId(salt.id.clone()),
        key_salt: salt.key_salt.as_ref().map(|s| KeySalt(s.to_string())),
    }
}

fn unlock_address_key_with_key<T: PGPProviderSync>(
    provider: &T,
    decryption_key: &PrivateKey,
    public_keys: &[T::PublicKey],
    address_key: &AddressKey,
    token: &str,
) -> Result<UnlockedAddressKey> {
    let decryptor = provider.new_decryptor();
    let as_private_key = provider
        .private_key_import_unlocked(decryption_key.as_ref(), DataEncoding::Bytes)
        .context("Error importing private user key")?;

    let decrypted_token = decryptor
        .with_decryption_key(&as_private_key)
        .with_verification_keys(public_keys)
        .decrypt(token, DataEncoding::Armor)
        .context("Error decrypting and verifying address key token")?;

    let instance = unlock_address_key_with_passphrase(provider, address_key, decrypted_token)?;
    Ok(instance)
}

fn unlock_address_key_with_passphrase<T: PGPProviderSync>(
    provider: &T,
    address_key: &AddressKey,
    passphrase: T::VerifiedData,
) -> Result<UnlockedAddressKey> {
    let as_private_key = provider
        .private_key_import(&address_key.private_key, passphrase, DataEncoding::Auto)
        .context("Error importing private address key for unlock with passphrase")?;

    let exported = provider
        .private_key_export_unlocked(&as_private_key, DataEncoding::Bytes)
        .context("Error exporting private key")?;

    Ok(UnlockedAddressKey {
        id: address_key.id.clone(),
        private_key: PrivateKey::new(exported.as_ref().to_vec()),
    })
}
