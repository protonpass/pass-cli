use crate::PassClient;
use anyhow::{Context, Result, anyhow};
use muon::GET;
use muon::rest::core::v4::addresses;
use pass_domain::{Address, AddressId, AddressKey, AddressKeyId};

struct AddressCacheType;

fn api_address_to_domain_address(value: addresses::Address) -> Address {
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

impl PassClient {
    pub async fn get_addresses(&self) -> Result<Vec<Address>> {
        {
            let cached: Option<Vec<Address>> = self.cache.get(AddressCacheType).await;
            if let Some(cached) = cached {
                return Ok(cached);
            }
        }

        let res = self
            .client
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
