use crate::{PassClient, PassClientContext};
use anyhow::{Context, Result, anyhow};
use muon::GET;
use pass_domain::{KeyPassphrase, KeyPassphrases, KeySalt, Passphrase};
use std::collections::HashMap;
use std::path::Path;

const PASSPHRASES_FILE_NAME: &str = "passphrases.enc";

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub(crate) struct GetKeySaltsResponse {
    #[serde(default, rename = "KeySalts")]
    pub key_salts: Vec<KeySaltResponse>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct KeySaltResponse {
    #[serde(rename = "ID")]
    pub id: String,

    #[serde(rename = "KeySalt")]
    pub key_salt: Option<String>,
}

impl From<KeySaltResponse> for KeySalt {
    fn from(value: KeySaltResponse) -> Self {
        Self {
            id: value.id,
            key_salt: value.key_salt,
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
struct SerializedKeyPassphrase {
    pub key_id: String,
    pub passphrase: String,
}

impl<C: PassClientContext> PassClient<C> {
    pub(crate) async fn setup_key_passphrases(&self, password: &str) -> Result<KeyPassphrases> {
        let salts = self.fetch_salts().await.context("failed to fetch salts")?;

        let account_crypto = self.client_features.get_account_crypto().await;
        let passphrases = account_crypto
            .generate_passphrases(salts, password)
            .await
            .context("failed to generate passphrases")?;
        self.store_passphrases(passphrases)
            .await
            .context("failed to store passphrases")
    }

    pub(crate) async fn setup_key_passphrases_with_passphrase(
        &self,
        passphrase: &[u8],
    ) -> Result<KeyPassphrases> {
        // Load user keys in order to prepare the HashMap for KeyID->Passphrase
        let user_keys = self
            .load_user_keys()
            .await
            .context("failed to load user keys")?;

        let mut passphrases = HashMap::new();
        for user_key in user_keys {
            passphrases.insert(user_key.id, Passphrase::new(passphrase.to_vec()));
        }
        self.store_passphrases(passphrases)
            .await
            .context("failed to store passphrases")
    }

    async fn store_passphrases(
        &self,
        passphrases: HashMap<String, Passphrase>,
    ) -> Result<KeyPassphrases> {
        let mut res = Vec::with_capacity(passphrases.len());
        let mut to_serialize = Vec::with_capacity(passphrases.len());
        for (key_id, passphrase) in passphrases {
            to_serialize.push(SerializedKeyPassphrase {
                key_id: key_id.to_string(),
                passphrase: crate::utils::b64_encode(passphrase.as_ref()),
            });

            res.push(KeyPassphrase {
                id: key_id,
                passphrase,
            })
        }

        let serialized =
            serde_json::to_vec(&to_serialize).context("failed to serialize passphrases")?;
        let encrypted = self
            .encrypt_with_local_key(&serialized)
            .await
            .context("failed to encrypt passphrases")?;

        let fs = self.client_features.get_fs().await;
        fs.store_file(encrypted, Path::new(PASSPHRASES_FILE_NAME))
            .await
            .context("failed to store passphrases")?;

        Ok(KeyPassphrases::new(res))
    }

    pub(crate) async fn get_key_passphrases(&self) -> Result<KeyPassphrases> {
        let fs = self.client_features.get_fs().await;
        let exists = fs
            .file_exists(Path::new(PASSPHRASES_FILE_NAME))
            .await
            .context("failed to check if passphrases file exists")?;

        if !exists {
            return Err(anyhow::anyhow!("Passphrases file not found"));
        }

        let contents = fs
            .get_file(Path::new(PASSPHRASES_FILE_NAME))
            .await
            .context("failed to get passphrases file")?;
        let decrypted = self
            .decrypt_with_local_key(&contents)
            .await
            .context("failed to decrypt passphrases")?;
        let passphrases: Vec<SerializedKeyPassphrase> =
            serde_json::from_slice(&decrypted).context("failed to decrypt passphrases")?;

        let mut res = Vec::with_capacity(passphrases.len());
        for passphrase in passphrases {
            let decoded = crate::utils::b64_decode(&passphrase.passphrase)
                .context("failed to decode passphrase")?;
            res.push(KeyPassphrase {
                id: passphrase.key_id,
                passphrase: Passphrase::new(decoded),
            })
        }

        Ok(KeyPassphrases::new(res))
    }

    async fn fetch_salts(&self) -> Result<Vec<KeySalt>> {
        let res = self.send(GET!("/core/v4/keys/salts")).await?;
        if !res.status().is_success() {
            return Err(anyhow!("HTTP Status: {:?}", res.status()));
        }
        let res: GetKeySaltsResponse = res.ok()?.into_body_json()?;

        let mapped = res.key_salts.into_iter().map(KeySalt::from).collect();
        Ok(mapped)
    }
}
