use crate::PassClient;
use crate::common::CodeResponse;
use anyhow::{Context, Result};
use muon::GET;
use pass_domain::PublicKey;

const UNPROCESSABLE_ENTITY_CODE: u16 = 422;
const ADDRESS_NOT_EXISTS_CODE: u32 = 33102;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub(crate) struct ActivePublicKeysResponse {
    #[serde(rename = "Address")]
    pub(crate) address: AddressDataResponse,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub(crate) struct AddressDataResponse {
    #[serde(rename = "Keys")]
    pub(crate) keys: Vec<PublicAddressKeyResponse>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub(crate) struct PublicAddressKeyResponse {
    #[serde(rename = "PublicKey")]
    pub(crate) public_key: String,
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
            .send(req)
            .await
            .context("Error sending get keys request")?;

        if res.status().as_u16() == UNPROCESSABLE_ENTITY_CODE {
            let body: CodeResponse = res.body_json().context("Error parsing response")?;
            if body.code == ADDRESS_NOT_EXISTS_CODE {
                return Ok(vec![]);
            }

            Err(anyhow::anyhow!("Error fetching keys for address"))
        } else {
            let response: ActivePublicKeysResponse = assert_response!(res);

            let mut result = vec![];
            let pgp = self.client_features.get_pgp_crypto().await;
            for response in response.address.keys {
                let unarmored = pgp
                    .unarmor(response.public_key)
                    .await
                    .context("Error unarmoring public key")?;
                result.push(PublicKey::new(unarmored));
            }

            Ok(result)
        }
    }
}
