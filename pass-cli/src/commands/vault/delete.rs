use super::VaultQuery;
use crate::helpers::CliPassClient as PassClient;
use anyhow::{Context, Result};

pub async fn run(client: PassClient, query: VaultQuery) -> Result<()> {
    let share_id = query.resolve(&client).await?;
    client
        .delete_vault(&share_id)
        .await
        .context("Error deleting vault")?;
    println!("Vault deleted successfully");
    Ok(())
}
