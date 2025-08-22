use crate::PassClient;
use crate::crypto::share_key::{OpenShareKeyFlow, OpenShareKeyForGroupFlow};
use crate::share::ShareKey;
use anyhow::{Context, Result, anyhow};
use pass_domain::{AddressId, GroupId, Share, ShareId};
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub(crate) struct DecryptedShareKey(pub(crate) Vec<u8>);

impl DecryptedShareKey {
    pub fn value(self) -> Vec<u8> {
        self.0.clone()
    }
}

impl AsRef<[u8]> for DecryptedShareKey {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl PassClient {
    pub(crate) async fn open_share_key_for_share_id(
        &self,
        share_id: &ShareId,
        key: ShareKey,
    ) -> Result<DecryptedShareKey> {
        let share = self
            .get_share(share_id)
            .await
            .context("Error getting share")?;
        self.open_share_key_for_share(&share, key).await
    }

    pub(crate) async fn open_share_key_for_share(
        &self,
        share: &Share,
        key: ShareKey,
    ) -> Result<DecryptedShareKey> {
        match share.group_id {
            None => self
                .open_share_key_for_direct_share(key)
                .await
                .context("Error opening ShareKey for Share"),
            Some(ref group_id) => self
                .open_share_key_from_group(&share.address_id, group_id, key)
                .await
                .context("Error opening ShareKey for Share via Group"),
        }
    }

    async fn open_share_key_for_direct_share(&self, key: ShareKey) -> Result<DecryptedShareKey> {
        let uks = self.get_user_keys().await?;
        let pgp_crypto = self.client_features.get_pgp_crypto().await;

        let flow = OpenShareKeyFlow::new(pgp_crypto, uks);
        let share_key = flow.open(key).await.context("failed to open ShareKey")?;
        Ok(DecryptedShareKey(share_key))
    }

    pub(crate) async fn open_share_key_from_group(
        &self,
        address: &AddressId,
        group_id: &GroupId,
        key: ShareKey,
    ) -> Result<DecryptedShareKey> {
        let invited_address = self
            .get_address(address)
            .await
            .context("Failed to get address")?;
        let address_keys = self
            .open_address_keys(invited_address.keys)
            .await
            .context("Failed to open address keys")?;
        let group_addresses = self
            .get_group_addresses()
            .await
            .context("Failed to fetch groups")?;
        let group_address = group_addresses
            .into_iter()
            .find(|g| g.group_id.eq(group_id))
            .ok_or_else(|| anyhow!("Could not find invited group"))?;

        let group_public_keys = self
            .get_keys_for_email(&group_address.address.email, false)
            .await
            .context("Error getting public keys for group")?;

        let pgp_crypto = self.client_features.get_pgp_crypto().await;
        let flow = OpenShareKeyForGroupFlow::new(pgp_crypto, address_keys, group_public_keys);
        let share_key = flow.open(key).await.context("failed to open ShareKey")?;
        Ok(DecryptedShareKey(share_key))
    }
}
