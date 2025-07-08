use crate::ClientFeatures;
use crate::cache::Cache;
use muon::Client;
use std::sync::Arc;

#[derive(Clone)]
pub struct PassClient {
    pub(crate) client: Client,
    pub(crate) cache: Cache,
    pub(crate) client_features: Arc<dyn ClientFeatures>,
}

impl PassClient {
    pub fn new(client: Client, client_features: Arc<dyn ClientFeatures>) -> Self {
        Self {
            client,
            client_features,
            cache: Cache::new(),
        }
    }
}
