use crate::PassClient;
use crate::utils::debug_response;
use anyhow::{Context, Result, anyhow};
use muon::DELETE;
use pass_domain::ShareId;

impl PassClient {
    pub async fn delete_vault(&self, share_id: &ShareId) -> Result<()> {
        let res = self
            .client
            .send(DELETE!("/pass/v1/vault/{}", share_id))
            .await
            .context("Failed to send delete Vault request")?;

        if !res.status().is_success() {
            debug_response(&res);
            return Err(anyhow!("Error in delete Vault request: {}", res.status()));
        }

        self.clear_shares_cache().await;
        Ok(())
    }
}
