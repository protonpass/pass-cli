use crate::PassClient;
use anyhow::Result;

impl PassClient {
    pub async fn logout(&self) -> Result<()> {
        self.client.logout().await;
        Ok(())
    }
}
