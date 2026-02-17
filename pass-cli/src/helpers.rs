use anyhow::{Result, anyhow};
use pass::PassClient;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::features::CliClientFeatures;
use pass_auth::PassSessionStore;

#[async_trait::async_trait]
pub trait SessionExt {
    async fn get_user_id(&self) -> Result<String>;
}

#[async_trait::async_trait]
impl SessionExt for Arc<RwLock<PassSessionStore>> {
    async fn get_user_id(&self) -> Result<String> {
        let store_guard = self.read().await;
        let auth = store_guard.auth.read().await;
        let user_id = auth
            .clone()
            .and_then(|a| a.user_id().map(|u| u.to_string()));
        match user_id {
            Some(user_id) => Ok(user_id),
            None => Err(anyhow!("Invalid current session: Does not have a UserID")),
        }
    }
}

pub trait PassClientExt {
    fn get_cli_client_features(&self) -> Result<CliClientFeatures>;
}

impl PassClientExt for PassClient {
    fn get_cli_client_features(&self) -> Result<CliClientFeatures> {
        let features = self.get_client_features();

        // HACK: Convert to &dyn Any so we can downcast it
        let any_ref = features.as_ref() as &dyn std::any::Any;

        any_ref
            .downcast_ref::<CliClientFeatures>()
            .cloned()
            .ok_or_else(|| anyhow!("Failed to downcast ClientFeatures to CliClientFeatures"))
    }
}
