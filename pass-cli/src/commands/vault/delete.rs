use anyhow::{Context, Result};
use pass::PassClient;
use pass_domain::ShareId;

pub async fn run(client: PassClient, share_id: ShareId) -> Result<()> {
    client
        .delete_vault(&share_id)
        .await
        .context("Error deleting vault")?;
    println!("Vault deleted successfully");
    Ok(())
}
