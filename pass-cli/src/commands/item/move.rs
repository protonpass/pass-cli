use crate::commands::item::view::{ItemQuery, ShareQuery};
use anyhow::{Context, Result, anyhow};
use pass::PassClient;
use pass_domain::{ItemId, ShareId};

pub struct MoveItemQuery {
    from_share_query: ShareQuery,
    item_query: ItemQuery,
    to_share_query: ShareQuery,
}

impl MoveItemQuery {
    pub fn new(
        from_share_id: Option<String>,
        from_vault: Option<String>,
        item_id: Option<String>,
        item_title: Option<String>,
        to_share_id: Option<String>,
        to_vault_name: Option<String>,
    ) -> Result<Self> {
        let from_share_query = match (from_share_id, from_vault) {
            (Some(share_id), None) => ShareQuery::ShareId(ShareId::new(share_id)),
            (None, Some(vault_name)) => ShareQuery::VaultName(vault_name),
            (None, None) => {
                return Err(anyhow!(
                    "Please provide either --from-share-id or --from-vault"
                ));
            }
            (Some(_), Some(_)) => {
                return Err(anyhow!(
                    "Please provide either --from-share-id or --from-vault, not both"
                ));
            }
        };

        let item_query = match (item_id, item_title) {
            (Some(item_id), None) => ItemQuery::ItemId(ItemId::new(item_id)),
            (None, Some(item_title)) => ItemQuery::ItemTitle(item_title),
            (None, None) => return Err(anyhow!("Please provide either --item-id or --item-title")),
            (Some(_), Some(_)) => {
                return Err(anyhow!(
                    "Please provide either --item-id or --item-title, not both"
                ));
            }
        };

        let to_share_query = match (to_share_id, to_vault_name) {
            (Some(share_id), None) => ShareQuery::ShareId(ShareId::new(share_id)),
            (None, Some(vault_name)) => ShareQuery::VaultName(vault_name),
            (None, None) => {
                return Err(anyhow!(
                    "Please provide either --to-share-id or --to-vault-name"
                ));
            }
            (Some(_), Some(_)) => {
                return Err(anyhow!(
                    "Please provide either --to-share-id or --to-vault-name, not both"
                ));
            }
        };

        Ok(Self {
            from_share_query,
            item_query,
            to_share_query,
        })
    }
}

pub async fn run(client: PassClient, query: MoveItemQuery) -> Result<()> {
    // Resolve source share_id
    let from_share_id = match query.from_share_query {
        ShareQuery::ShareId(share_id) => share_id.clone(),
        ShareQuery::VaultName(from_vault_name) => {
            let vault = client
                .find_vault(&from_vault_name)
                .await
                .context("Error finding source vault")?;
            vault.share_id
        }
    };

    // Resolve item id
    let item_id = match query.item_query {
        ItemQuery::ItemId(id) => id,
        ItemQuery::ItemTitle(title) => {
            let items = client
                .list_items(&from_share_id)
                .await
                .context("Error listing items")?;

            let matching_item = items
                .iter()
                .find(|item| item.content.title == title)
                .ok_or_else(|| anyhow!("No item found with title: {}", title))?;

            matching_item.id.clone()
        }
    };

    // Resolve destination share_id
    let to_share_id = match query.to_share_query {
        ShareQuery::ShareId(id) => id,
        ShareQuery::VaultName(vault_name) => {
            let vault = client
                .find_vault(&vault_name)
                .await
                .context("Error finding destination vault")?;
            vault.share_id
        }
    };

    let new_item_id = client
        .move_item(&from_share_id, &item_id, &to_share_id)
        .await
        .context("Error moving item")?;

    println!("{new_item_id}");
    Ok(())
}
