use crate::commands::Role;
use anyhow::{Context, Result};
use pass::PassClient;
use pass_domain::{ShareId, ShareRole};

pub async fn run(client: PassClient, share_id: ShareId, email: String, role: Role) -> Result<()> {
    client
        .share_vault(&share_id, &email, &ShareRole::from(role))
        .await
        .context("Error sharing vault")?;

    Ok(())
}
