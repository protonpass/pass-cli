use crate::{PassClient, PassClientContext};
use anyhow::{Context, Result, anyhow};
use muon::GET;
use muon::rest::core::v4::addresses;
use pass_domain::{Address, AddressId, AddressKey, AddressKeyId};

struct AddressCacheType;

pub(crate) fn api_address_to_domain_address(value: addresses::Address) -> Address {
    Address {
        id: AddressId::new(value.id),
        email: value.email,
        keys: value
            .keys
            .into_iter()
            .map(|ak| AddressKey {
                id: AddressKeyId::new(ak.id),
                active: ak.active.into(),
                primary: ak.primary.into(),
                private_key: ak.private_key,
                token: ak.token,
                signature: ak.signature,
            })
            .collect(),
    }
}

impl<C: PassClientContext> PassClient<C> {
    pub async fn get_addresses(&self) -> Result<Vec<Address>> {
        {
            let cached: Option<Vec<Address>> = self.cache.get(AddressCacheType).await;
            if let Some(cached) = cached {
                return Ok(cached);
            }
        }

        let res = self
            .send(GET!("/addresses"))
            .await
            .context("Error requesting addresses")?;
        if !res.status().is_success() {
            return Err(anyhow!("HTTP Status: {:?}", res.status()));
        }

        let response: addresses::GetRes = res
            .body_json()
            .context("Error parsing get addresses response")?;

        let mut res = Vec::with_capacity(response.addresses.len());
        for address in response.addresses {
            res.push(api_address_to_domain_address(address));
        }

        self.cache.store(AddressCacheType, res.clone()).await;

        Ok(res)
    }

    pub async fn get_address(&self, address_id: &AddressId) -> Result<Address> {
        let addresses = self
            .get_addresses()
            .await
            .context("Error retrieving addresses")?;
        for address in addresses {
            if address.id.eq(address_id) {
                return Ok(address);
            }
        }

        Err(anyhow!("Address not found"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_tools::*;

    #[muon_test::test]
    async fn can_handle_empty_addresses(s: muon_test::Server) {
        let (raw_client, api) = s.client::<()>();
        api.handler("/addresses", |_| {
            success(addresses::GetRes { addresses: vec![] })
        });

        let client = make_test_pass_client(raw_client, &api).await;
        let addresses = client
            .get_addresses()
            .await
            .expect("Should be able to get addresses");
        assert!(addresses.is_empty());
    }

    #[muon_test::test]
    async fn can_handle_addresses(s: muon_test::Server) {
        let (raw_client, api) = s.client::<()>();
        const A1_ID: &str = "address1";
        const A1_EMAIL: &str = "address1@test.com";
        const A2_ID: &str = "address2";
        const A2_EMAIL: &str = "address2@test.com";

        api.handler("/addresses", |_| {
            success(addresses::GetRes {
                addresses: vec![
                    addresses::Address {
                        id: A1_ID.to_string(),
                        email: A1_EMAIL.to_string(),
                        keys: vec![],
                    },
                    addresses::Address {
                        id: A2_ID.to_string(),
                        email: A2_EMAIL.to_string(),
                        keys: vec![],
                    },
                ],
            })
        });

        let client = make_test_pass_client(raw_client, &api).await;
        let addresses = client
            .get_addresses()
            .await
            .expect("Should be able to get addresses");
        assert_eq!(addresses.len(), 2);
        assert_eq!(A1_ID, addresses[0].id.value());
        assert_eq!(A1_EMAIL, addresses[0].email);
        assert_eq!(A2_ID, addresses[1].id.value());
        assert_eq!(A2_EMAIL, addresses[1].email);
    }

    #[muon_test::test]
    async fn caches_addresses(s: muon_test::Server) {
        let (raw_client, api) = s.client::<()>();
        const A1_ID: &str = "address1";
        const A1_EMAIL: &str = "address1@test.com";

        api.handler("/addresses", |_| {
            success(addresses::GetRes {
                addresses: vec![addresses::Address {
                    id: A1_ID.to_string(),
                    email: A1_EMAIL.to_string(),
                    keys: vec![],
                }],
            })
        });

        let client = make_test_pass_client(raw_client, &api).await;

        let recorder = api.new_recorder();

        // First request
        client
            .get_addresses()
            .await
            .expect("Should be able to get addresses");

        let requests_1 = recorder.read().len();

        // Second request
        client
            .get_addresses()
            .await
            .expect("Should be able to get addresses");

        let requests_2 = recorder.read().len();

        assert_eq!(requests_1, requests_2);
    }
}
