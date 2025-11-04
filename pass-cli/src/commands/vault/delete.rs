use anyhow::{Context, Result, anyhow};
use pass::PassClient;
use pass_domain::ShareId;

pub enum DeleteVaultQuery {
    ShareId(ShareId),
    VaultName(String),
}

impl DeleteVaultQuery {
    pub fn new(share_id: Option<String>, name: Option<String>) -> Result<Self> {
        match (share_id, name) {
            (Some(share_id), None) => Ok(Self::ShareId(ShareId::new(share_id))),
            (None, Some(vault_name)) => Ok(Self::VaultName(vault_name)),

            _ => Err(anyhow!("Please provide either share-id or vault name")),
        }
    }
}

pub async fn run(client: PassClient, query: DeleteVaultQuery) -> Result<()> {
    let share_id = match query {
        DeleteVaultQuery::ShareId(id) => id,
        DeleteVaultQuery::VaultName(vault) => {
            let vault = client
                .find_vault(&vault)
                .await
                .context("Error finding vault")?;
            vault.share_id
        }
    };
    client
        .delete_vault(&share_id)
        .await
        .context("Error deleting vault")?;
    println!("Vault deleted successfully");
    Ok(())
}
