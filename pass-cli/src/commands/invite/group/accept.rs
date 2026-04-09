use crate::helpers::CliPassClient as PassClient;
use anyhow::{Context, Result};
use pass_domain::InviteId;

pub async fn run(client: PassClient, invite_id: InviteId) -> Result<()> {
    client
        .accept_group_invite(&invite_id)
        .await
        .context("Error accepting group invite")?;
    Ok(())
}
