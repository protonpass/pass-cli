use crate::{PassClient, PassClientContext};
use anyhow::{Context, Result};

impl<C: PassClientContext> PassClient<C> {
    pub async fn logout(&self) -> Result<()> {
        self.client
            .get_session(())
            .await
            .context("Error getting client session")?
            .logout()
            .await;
        Ok(())
    }
}
