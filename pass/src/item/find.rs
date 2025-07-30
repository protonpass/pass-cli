use crate::PassClient;
use anyhow::{Context, Result, anyhow};
use pass_domain::{Item, ItemId, ShareId};

#[derive(Debug)]
pub enum FindItemQuery {
    Name {
        vault_name: String,
        item_name: String,
    },
    Id {
        share_id: ShareId,
        item_id: ItemId,
    },
}

impl FindItemQuery {
    pub fn new(vault: &str, item: &str) -> Self {
        if Self::is_id(vault) && Self::is_id(item) {
            Self::Id {
                share_id: ShareId::new(vault.to_string()),
                item_id: ItemId::new(item.to_string()),
            }
        } else {
            Self::Name {
                vault_name: vault.to_string(),
                item_name: item.to_string(),
            }
        }
    }

    fn is_id(value: &str) -> bool {
        value.len() == 88 && value.ends_with("==")
    }
}

impl PassClient {
    pub async fn find_item(&self, query: FindItemQuery) -> Result<Item> {
        match query {
            FindItemQuery::Name {
                vault_name,
                item_name,
            } => self
                .find_item_by_name(&vault_name, &item_name)
                .await
                .context("Error finding item by name"),
            FindItemQuery::Id { share_id, item_id } => {
                let item = self
                    .view_item(&share_id, &item_id)
                    .await
                    .context("Error retrieving item by id")?;
                Ok(item.item)
            }
        }
    }

    async fn find_item_by_name(&self, vault_name: &str, item_name: &str) -> Result<Item> {
        let vaults = self.list_vaults().await.context("Error listing vaults")?;
        let vault = vaults
            .into_iter()
            .find(|v| v.content.name == vault_name)
            .ok_or_else(|| anyhow!("Could not find vault {}", vault_name))?;

        let items = self
            .list_items(&vault.share_id)
            .await
            .context("Error listing items")?;
        let item = items
            .into_iter()
            .find(|i| i.content.title == item_name)
            .ok_or_else(|| anyhow!("Could not find item with name {}", item_name))?;

        Ok(item)
    }
}
