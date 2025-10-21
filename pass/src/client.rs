use crate::cache::Cache;
use anyhow::{Context, Result};
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

    pub async fn perform_first_time_setup(&self, pass: &str) -> Result<()> {
        self.setup_key_passphrases(pass)
            .await
            .context("Error setting up key passphrases")?;

        Ok(())
    }

    pub(crate) async fn send(&self, req: muon::http::HttpReq) -> Result<muon::http::HttpRes> {
        self.client
            .get_session(())
            .await
            .context("Error getting client session")?
            .send(req)
            .await
            .context("Error sending request")
    }
}
