use super::super::VaultQuery;
use crate::commands::Role;
use crate::helpers::CliPassClient as PassClient;
use anyhow::{Context, Result};
use pass_domain::{ShareId, ShareRole};

pub async fn run(
    client: PassClient,
    query: VaultQuery,
    member_share_id: ShareId,
    role: Role,
) -> Result<()> {
    let share_id = query.resolve(&client).await?;
    let share_role: ShareRole = role.into();

    client
        .update_vault_member(&share_id, &member_share_id, share_role)
        .await
        .context("Error updating vault member")?;

    println!("Successfully updated member role");
    Ok(())
}
