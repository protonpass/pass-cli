use crate::helpers::CliPassClient as PassClient;
use anyhow::{Context, Result};
use pass_domain::{ItemId, ShareId};

pub async fn run(client: PassClient, share_id: ShareId, item_id: ItemId) -> Result<()> {
    client
        .delete_item(&share_id, &item_id)
        .await
        .context("Error deleting item")?;

    println!("Item {item_id} deleted successfully");
    Ok(())
}
