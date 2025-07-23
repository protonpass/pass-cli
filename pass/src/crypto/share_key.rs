use crate::share::ShareKey;
use crate::{PgpCrypto, PrivateKey, PublicKey, UserKey};
use anyhow::Result;
use std::sync::Arc;

pub(crate) struct OpenShareKeyFlow {
    pub crypto: Arc<dyn PgpCrypto>,
    pub user_keys: Vec<UserKey>,
}

impl OpenShareKeyFlow {
    pub fn new(crypto: Arc<dyn PgpCrypto>, user_keys: Vec<UserKey>) -> Self {
        Self { crypto, user_keys }
    }

    pub async fn open(self, vault_key: ShareKey) -> Result<Vec<u8>> {
        let mut private_keys = vec![];
        let mut public_keys = vec![];

        for user_key in self.user_keys {
            private_keys.push(PrivateKey {
                content: user_key.private_key,
            });
            public_keys.push(PublicKey {
                content: user_key.public_key,
            });
        }
        self.crypto
            .decrypt_and_verify(vault_key.key.0, private_keys, public_keys, None)
            .await
    }
}
