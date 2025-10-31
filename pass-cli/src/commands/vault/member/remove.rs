use anyhow::{Context, Result};
use pass::PassClient;
use pass_domain::ShareId;

pub async fn run(client: PassClient, share_id: ShareId, member_share_id: ShareId) -> Result<()> {
    client
        .remove_vault_member(&share_id, &member_share_id)
        .await
        .context("Error removing vault member")?;

    println!("Successfully removed member from vault");
    Ok(())
}
