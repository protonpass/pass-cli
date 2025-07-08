use crate::{PassClient, PublicKey};
use anyhow::{Context, Result};
use muon::GET;

#[derive(Debug, serde::Deserialize)]
struct ActivePublicKeysResponse {
    #[serde(rename = "Address")]
    address: AddressDataResponse,
    #[serde(rename = "CatchAll")]
    catch_all: Option<AddressDataResponse>,
    #[serde(rename = "Unverified")]
    unverified: Option<AddressDataResponse>,
    #[serde(rename = "Warnings")]
    warnings: Vec<String>,
    #[serde(rename = "ProtonMX")]
    proton_mx: bool,
    #[serde(rename = "IsProton")]
    is_proton: u8,
}

#[derive(Debug, serde::Deserialize)]
struct AddressDataResponse {
    #[serde(rename = "Keys")]
    keys: Vec<PublicAddressKeyResponse>,
}

#[derive(Debug, serde::Deserialize)]
struct PublicAddressKeyResponse {
    #[serde(rename = "Flags")]
    flags: u32,
    #[serde(rename = "PublicKey")]
    public_key: String,
    #[serde(rename = "Source")]
    source: Option<u8>,
}

impl PassClient {
    pub(crate) async fn get_keys_for_email(
        &self,
        address: &str,
        internal_only: bool,
    ) -> Result<Vec<PublicKey>> {
        let internal_only_value = match internal_only {
            true => 1,
            false => 0,
        };
        let req = GET!("/core/v4/keys/all")
            .query(("Email", address))
            .query(("InternalOnly", internal_only_value.to_string()));

        let res = self
            .client
            .send(req)
            .await
            .context("Error sending get keys request")?;
        let response: ActivePublicKeysResponse = assert_response!(res);

        let mut result = vec![];
        let pgp = self.client_features.get_pgp_crypto().await;
        for response in response.address.keys {
            let unarmored = pgp
                .unarmor(response.public_key)
                .await
                .context("Error unarmoring public key")?;
            result.push(PublicKey { content: unarmored });
        }

        Ok(result)
    }
}
