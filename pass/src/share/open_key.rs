use crate::PassClient;
use crate::crypto::share_key::OpenShareKeyFlow;
use crate::share::ShareKey;
use anyhow::{Context, Result};

#[derive(Clone)]
pub(crate) struct DecryptedShareKey(pub(crate) Vec<u8>);

impl DecryptedShareKey {
    pub fn value(self) -> Vec<u8> {
        self.0
    }
}

impl AsRef<[u8]> for DecryptedShareKey {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl PassClient {
    pub(crate) async fn open_share_key(&self, key: ShareKey) -> Result<DecryptedShareKey> {
        let uks = self.get_user_keys().await?;
        let pgp_crypto = self.client_features.get_pgp_crypto().await;

        let flow = OpenShareKeyFlow::new(pgp_crypto, uks);
        let share_key = flow.open(key).await.context("failed to open ShareKey")?;
        Ok(DecryptedShareKey(share_key))
    }
}
