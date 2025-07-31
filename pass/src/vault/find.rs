use crate::PassClient;
use anyhow::{Context, Result, anyhow};
use pass_domain::Vault;

impl PassClient {
    pub async fn find_vault(&self, vault_name: &str) -> Result<Vault> {
        let vaults = self.list_vaults().await.context("Error listing vaults")?;
        let vault = vaults
            .into_iter()
            .find(|v| v.content.name == vault_name)
            .ok_or_else(|| anyhow!("Could not find vault {}", vault_name))?;

        Ok(vault)
    }
}
