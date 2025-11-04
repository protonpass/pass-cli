use super::VaultQuery;
use crate::commands::Role;
use anyhow::{Context, Result};
use pass::PassClient;
use pass_domain::ShareRole;

pub async fn run(client: PassClient, query: VaultQuery, email: String, role: Role) -> Result<()> {
    let share_id = query.resolve(&client).await?;
    client
        .share_vault(&share_id, &email, &ShareRole::from(role))
        .await
        .context("Error sharing vault")?;

    Ok(())
}
