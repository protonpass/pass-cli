use crate::PassClient;
use anyhow::{Context, Result};

impl PassClient {
    pub async fn logout(&self) -> Result<()> {
        self.client
            .get_session(())
            .await
            .context("Error getting clienet session")?
            .logout()
            .await;
        Ok(())
    }
}
