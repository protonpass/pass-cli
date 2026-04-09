use crate::common::CodeResponse;
use crate::{PassClient, PassClientContext};
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

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub(crate) struct PublicAddressKeyResponse {
    #[serde(rename = "PublicKey")]
    pub(crate) public_key: String,
    #[serde(rename = "Primary")]
    pub(crate) primary: u8,
}

impl PublicAddressKeyResponse {
    pub fn reorder_with_primary_first(vec: Vec<Self>) -> Vec<Self> {
        let mut primary_vec = Vec::with_capacity(vec.len());

        if let Some(pos) = vec.iter().position(|item| item.primary == 1) {
            // Move the primary key as the first one
            primary_vec.push(vec[pos].clone());
            // Then push the rest in the same order, skipping the primary one
            primary_vec.extend(
                vec.into_iter().enumerate().filter_map(
                    |(i, item)| {
                        if i == pos { None } else { Some(item) }
                    },
                ),
            );
        } else {
            // No primary found, just return as-is
            return vec;
        }

        primary_vec
    }
}

impl<C: PassClientContext> PassClient<C> {
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
            let keys = PublicAddressKeyResponse::reorder_with_primary_first(response.address.keys);

            let mut result = vec![];
            let pgp = self.client_features.get_pgp_crypto().await;
            for response in keys {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_reorder_puts_primary_first() {
        let non_primary_1 = PublicAddressKeyResponse {
            primary: 0,
            public_key: "A".to_string(),
        };
        let non_primary_2 = PublicAddressKeyResponse {
            primary: 0,
            public_key: "B".to_string(),
        };
        let primary = PublicAddressKeyResponse {
            primary: 1,
            public_key: "C".to_string(),
        };

        let keys = vec![
            non_primary_1.clone(),
            primary.clone(),
            non_primary_2.clone(),
        ];
        let res = PublicAddressKeyResponse::reorder_with_primary_first(keys);

        assert_eq!(res.len(), 3);
        assert_eq!(1, res[0].primary);
        assert_eq!(primary.public_key, res[0].public_key);

        assert_eq!(non_primary_1.public_key, res[1].public_key);
        assert_eq!(non_primary_2.public_key, res[2].public_key);
    }
    #[test]
    fn check_reorder_puts_primary_first_even_if_it_was_first() {
        let non_primary_1 = PublicAddressKeyResponse {
            primary: 0,
            public_key: "A".to_string(),
        };
        let non_primary_2 = PublicAddressKeyResponse {
            primary: 0,
            public_key: "B".to_string(),
        };
        let primary = PublicAddressKeyResponse {
            primary: 1,
            public_key: "C".to_string(),
        };

        let keys = vec![
            primary.clone(),
            non_primary_1.clone(),
            non_primary_2.clone(),
        ];
        let res = PublicAddressKeyResponse::reorder_with_primary_first(keys);

        assert_eq!(res.len(), 3);
        assert_eq!(1, res[0].primary);
        assert_eq!(primary.public_key, res[0].public_key);

        assert_eq!(non_primary_1.public_key, res[1].public_key);
        assert_eq!(non_primary_2.public_key, res[2].public_key);
    }
}
