use super::VaultQuery;
use anyhow::{Context, Result};
use pass::PassClient;

pub async fn run(client: PassClient, query: VaultQuery) -> Result<()> {
    let share_id = query.resolve(&client).await?;
    client
        .delete_vault(&share_id)
        .await
        .context("Error deleting vault")?;
    println!("Vault deleted successfully");
    Ok(())
}
