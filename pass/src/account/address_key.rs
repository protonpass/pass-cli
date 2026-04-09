use crate::{PassClient, PassClientContext};
use anyhow::{Context, Result};
use pass_domain::{AddressKey, UnlockedAddressKeys};

impl<C: PassClientContext> PassClient<C> {
    pub async fn open_address_keys(
        &self,
        address_keys: Vec<AddressKey>,
    ) -> Result<UnlockedAddressKeys> {
        let user_keys = self
            .get_user_keys()
            .await
            .context("Error getting user keys")?;

        let account_crypto = self.client_features.get_account_crypto().await;
        account_crypto
            .open_address_keys(user_keys, address_keys)
            .await
            .context("Error opening address keys")
    }
}
