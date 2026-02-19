use crate::cache::Cache;
use crate::error::SessionInvalidatedError;
use crate::muon_ext::MuonErrorExt;
use anyhow::{Context, Result};
use muon::Session;
use pass_domain::{AccountType, ClientFeatures};
use std::sync::Arc;
use tracing::warn;

pub type PassSessionKeyType = ();
pub type Client = muon::Client<PassSessionKeyType>;

#[derive(Clone)]
pub struct PassClient {
    pub(crate) client: Client,
    pub(crate) cache: Cache,
    pub(crate) client_features: Arc<dyn ClientFeatures>,
    pub(crate) memory_xor_key: u8,
    pub(crate) account_type: AccountType,
}

impl PassClient {
    pub fn new(
        client: Client,
        client_features: Arc<dyn ClientFeatures>,
        account_type: AccountType,
    ) -> Self {
        Self {
            client,
            client_features,
            cache: Cache::new(),
            memory_xor_key: pass_domain::crypto::generate_random_byte(),
            account_type,
        }
    }

    pub fn account_type(&self) -> AccountType {
        self.account_type
    }

    pub async fn get_key_provider(&self) -> Result<Arc<dyn pass_domain::LocalKeyProvider>> {
        self.client_features.get_local_key_provider().await
    }

    pub(crate) async fn send(&self, req: muon::http::HttpReq) -> Result<muon::http::HttpRes> {
        match self.get_session().await?.send(req).await {
            Ok(r) => Ok(r),
            Err(e) => {
                if e.is_logged_out_error() {
                    warn!("Session has been invalidated by the server, clearing local data");
                    if let Err(cleanup_err) = self.client_features.on_session_invalidated().await {
                        warn!(
                            "Failed to clear local data after session invalidation: {cleanup_err:#}"
                        );
                    }
                    Err(anyhow::Error::new(SessionInvalidatedError))
                } else {
                    Err(e).context("Error sending request")
                }
            }
        }
    }

    pub async fn get_session(&self) -> Result<Session<PassSessionKeyType>> {
        self.client
            .get_session(())
            .await
            .context("Error getting client session")
    }

    pub fn get_client_features(&self) -> Arc<dyn ClientFeatures> {
        self.client_features.clone()
    }
}
