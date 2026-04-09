use anyhow::{Result, anyhow};
use pass::{PassClient, PassClientContext};
use pass_auth::os::ProdContext;
use std::sync::Arc;
use std::sync::RwLock;

use crate::features::CliClientFeatures;
use pass_auth::PassSessionStore;

#[async_trait::async_trait]
pub trait SessionExt {
    async fn get_user_id(&self) -> Result<String>;
}

#[async_trait::async_trait]
impl SessionExt for Arc<RwLock<PassSessionStore>> {
    async fn get_user_id(&self) -> Result<String> {
        let store_guard = self.read().expect("store rwlock poisoned");
        let auth = store_guard.auth.lock().expect("auth mutex poisoned");
        let user_id = auth
            .as_ref()
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

impl<C: PassClientContext> PassClientExt for PassClient<C> {
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

/// Type alias for the concrete PassClient used in the CLI
pub type CliPassClient = PassClient<ProdContext>;
