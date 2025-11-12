use crate::cache::Cache;
use anyhow::{Context, Result};
use muon::Session;
use pass_domain::ClientFeatures;
use std::sync::Arc;

pub type PassSessionKeyType = ();
pub type Client = muon::Client<PassSessionKeyType>;

#[derive(Clone)]
pub struct PassClient {
    pub(crate) client: Client,
    pub(crate) cache: Cache,
    pub(crate) client_features: Arc<dyn ClientFeatures>,
    pub(crate) memory_xor_key: u8,
}

impl PassClient {
    pub fn new(client: Client, client_features: Arc<dyn ClientFeatures>) -> Self {
        Self {
            client,
            client_features,
            cache: Cache::new(),
            memory_xor_key: pass_domain::crypto::generate_random_byte(),
        }
    }

    pub async fn get_key_provider(&self) -> Result<Arc<dyn pass_domain::LocalKeyProvider>> {
        self.client_features.get_local_key_provider().await
    }

    pub(crate) async fn send(&self, req: muon::http::HttpReq) -> Result<muon::http::HttpRes> {
        self.get_session()
            .await?
            .send(req)
            .await
            .context("Error sending request")
    }

    pub async fn get_session(&self) -> Result<Session<PassSessionKeyType>> {
        self.client
            .get_session(())
            .await
            .context("Error getting client session")
    }
}
