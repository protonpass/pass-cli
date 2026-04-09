use super::common::{ItemQuery, ShareQuery};
use crate::helpers::CliPassClient as PassClient;
use anyhow::{Context, Result};

pub struct TrashItemQuery {
    share_query: ShareQuery,
    item_query: ItemQuery,
}

impl TrashItemQuery {
    pub fn new(
        share_id: Option<String>,
        vault_name: Option<String>,
        item_id: Option<String>,
        item_title: Option<String>,
    ) -> Result<Self> {
        let share_query = ShareQuery::new(share_id, vault_name)?;
        let item_query = ItemQuery::new(item_id, item_title)?;

        Ok(Self {
            share_query,
            item_query,
        })
    }
}

pub async fn run(client: PassClient, query: TrashItemQuery) -> Result<()> {
    let share_id = query.share_query.share_id(&client).await?;
    let item_id = query.item_query.item_id(&share_id, &client).await?;

    client
        .trash_item(&share_id, &item_id)
        .await
        .context("Error trashing item")?;

    println!("Item successfully moved to trash");
    Ok(())
}
