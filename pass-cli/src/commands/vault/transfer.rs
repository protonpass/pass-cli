use crate::commands::vault::VaultQuery;
use anyhow::{Context, Result};
use pass::PassClient;
use pass_domain::ShareId;

pub async fn run(client: PassClient, query: VaultQuery, member_share_id: ShareId) -> Result<()> {
    let share_id = query.resolve(&client).await?;
    client
        .transfer_ownership(&share_id, &member_share_id)
        .await
        .context("Error transferring vault ownership")?;

    Ok(())
}
