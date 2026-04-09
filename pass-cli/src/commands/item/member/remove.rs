use crate::helpers::CliPassClient as PassClient;
use anyhow::{Context, Result};
use pass_domain::ShareId;

pub async fn run(client: PassClient, share_id: ShareId, member_share_id: ShareId) -> Result<()> {
    client
        .remove_vault_member(&share_id, &member_share_id)
        .await
        .context("Error removing item member")?;

    println!("Successfully removed member from item");
    Ok(())
}
