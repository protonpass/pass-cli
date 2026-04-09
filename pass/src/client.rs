use crate::cache::Cache;
use crate::error::SessionInvalidatedError;
use crate::muon_ext::MuonErrorExt;
use anyhow::{Context, Result};
use muon::Session;
use muon::common;
use pass_domain::{AccountType, ClientFeatures};
use std::sync::Arc;
use tracing::warn;

pub type PassSessionKeyType = ();

pub trait PassClientContext: common::Context<SessionKey = PassSessionKeyType> {}
impl<C: common::Context<SessionKey = PassSessionKeyType>> PassClientContext for C {}

pub struct PassClient<C: PassClientContext = common::StubContext> {
    pub(crate) client: muon::Client<C>,
    pub(crate) cache: Cache,
    pub(crate) client_features: Arc<dyn ClientFeatures>,
    pub(crate) memory_xor_key: u8,
    pub(crate) account_type: AccountType,
}

impl<C: PassClientContext> Clone for PassClient<C> {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            cache: self.cache.clone(),
            client_features: self.client_features.clone(),
            memory_xor_key: self.memory_xor_key,
            account_type: self.account_type,
        }
    }
}

impl<C: PassClientContext> PassClient<C> {
    pub fn new(
        client: muon::Client<C>,
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

    pub fn is_user_account(&self) -> bool {
        self.account_type == AccountType::User
    }

    pub fn is_pat_account(&self) -> bool {
        self.account_type == AccountType::PersonalAccessToken
    }

    pub async fn get_key_provider(&self) -> Result<Arc<dyn pass_domain::LocalKeyProvider>> {
        self.client_features.get_local_key_provider().await
    }

    pub(crate) async fn send(&self, req: muon::http::HttpReq) -> Result<muon::http::HttpRes> {
        // GET requests are always safe to retry — mark them idempotent so muon v2
        // transparently retries on broken pipe (stale pooled connection).
        let req = if req.get_method() == muon::http::Method::GET {
            req.service_type(muon::common::ServiceType::Normal, true)
        } else {
            req
        };
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

    pub async fn get_session(&self) -> Result<Session<C>> {
        self.client
            .get_session(())
            .await
            .ok_or_else(|| anyhow::anyhow!("No active session"))
    }

    pub fn get_client_features(&self) -> Arc<dyn ClientFeatures> {
        self.client_features.clone()
    }
}
