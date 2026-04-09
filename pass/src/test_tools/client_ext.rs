use pass_domain::PlainText;

#[async_trait::async_trait(?Send)]
pub trait ClientTestExt {
    async fn encrypt_for_user_key(&self, data: Vec<u8>) -> Vec<u8>;
}

#[async_trait::async_trait(?Send)]
impl ClientTestExt for super::muon_ext::TestPassClient {
    async fn encrypt_for_user_key(&self, data: Vec<u8>) -> Vec<u8> {
        let user_key = self
            .get_primary_user_key()
            .await
            .expect("Error getting user key");
        let (private, public) = user_key.into_keys();
        let crypto = self.client_features.get_pgp_crypto().await;
        crypto
            .encrypt_and_sign(PlainText::new(data), public, private, None)
            .await
            .expect("Error encrypting data")
    }
}
