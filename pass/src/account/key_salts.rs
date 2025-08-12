use crate::PassClient;
use crate::account::{KeyPassphrase, KeyPassphrases, Passphrase};
use anyhow::{Context, Result, anyhow};
use muon::rest::core::v4::keys::salts::KeySalt;
use muon::{Client, GET};
use std::path::Path;

const PASSPHRASES_FILE_NAME: &str = "passphrases.enc";

#[derive(serde::Deserialize, serde::Serialize)]
struct SerializedKeyPassphrase {
    pub key_id: String,
    pub passphrase: String,
}

impl PassClient {
    pub(crate) async fn setup_key_passphrases(&self, password: &str) -> Result<KeyPassphrases> {
        let salts = fetch_salts(&self.client)
            .await
            .context("failed to fetch salts")?;

        let passphrases = self
            .client_features
            .generate_passphrases(salts, password)
            .await
            .context("failed to generate passphrases")?;

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
        self.client_features
            .store_file(encrypted, Path::new(PASSPHRASES_FILE_NAME))
            .await
            .context("failed to store passphrases")?;

        Ok(KeyPassphrases::new(res))
    }

    pub(crate) async fn get_key_passphrases(&self) -> Result<KeyPassphrases> {
        let exists = self
            .client_features
            .file_exists(Path::new(PASSPHRASES_FILE_NAME))
            .await
            .context("failed to check if passphrases file exists")?;

        if !exists {
            return Err(anyhow::anyhow!("Passphrases file not found"));
        }

        let contents = self
            .client_features
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
                passphrase: Passphrase(decoded),
            })
        }

        Ok(KeyPassphrases::new(res))
    }
}

async fn fetch_salts(client: &Client) -> Result<Vec<KeySalt>> {
    let res = client.send(GET!("/core/v4/keys/salts")).await?;
    if !res.status().is_success() {
        return Err(anyhow!("HTTP Status: {:?}", res.status()));
    }
    let res: muon::rest::core::v4::keys::salts::GetRes = res.ok()?.into_body_json()?;

    Ok(res.key_salts)
}
