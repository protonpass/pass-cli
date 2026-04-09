use super::super::VaultQuery;
use crate::helpers::CliPassClient as PassClient;
use anyhow::{Context, Result};
use pass_domain::ShareId;

pub async fn run(client: PassClient, query: VaultQuery, member_share_id: ShareId) -> Result<()> {
    let share_id = query.resolve(&client).await?;
    client
        .remove_vault_member(&share_id, &member_share_id)
        .await
        .context("Error removing vault member")?;

    println!("Successfully removed member from vault");
    Ok(())
}
