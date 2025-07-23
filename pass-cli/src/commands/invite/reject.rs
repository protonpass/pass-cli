use anyhow::{Context, Result};
use pass::PassClient;
use pass_domain::InviteId;

pub async fn run(client: PassClient, invite_id: InviteId) -> Result<()> {
    client
        .reject_invite(&invite_id)
        .await
        .context("Error rejecting invite")?;
    Ok(())
}
