use crate::PassClient;
use crate::account::UnlockedAddressKeys;
use anyhow::{Context, Result};
use pass_domain::AddressKey;

impl PassClient {
    pub async fn open_address_keys(
        &self,
        address_keys: Vec<AddressKey>,
    ) -> Result<UnlockedAddressKeys> {
        let user_keys = self
            .get_user_keys()
            .await
            .context("Error getting user keys")?;

        self.client_features
            .open_address_keys(user_keys, address_keys)
            .await
            .context("Error opening address keys")
    }
}
