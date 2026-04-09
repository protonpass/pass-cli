use crate::commands::Role;
use crate::helpers::CliPassClient as PassClient;
use anyhow::{Context, Result};
use pass_domain::{ItemId, ShareId, ShareRole};

pub async fn run(
    client: PassClient,
    share_id: ShareId,
    item_id: ItemId,
    address: &str,
    role: Role,
) -> Result<()> {
    client
        .share_item(&share_id, &item_id, address, &ShareRole::from(role))
        .await
        .context("Error sharing item")?;
    Ok(())
}
