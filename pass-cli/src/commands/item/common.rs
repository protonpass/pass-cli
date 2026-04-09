use crate::helpers::CliPassClient as PassClient;
use anyhow::{Context, Result, anyhow};
use pass_domain::{ItemId, ShareId};

pub enum ShareQuery {
    ShareId(ShareId),
    VaultName(String),
}

impl ShareQuery {
    pub fn new(share_id: Option<String>, vault_name: Option<String>) -> Result<Self> {
        Self::new_with_arg_name(share_id, vault_name, "--share-id", "--vault-name")
    }

    pub fn new_with_arg_name(
        share_id: Option<String>,
        vault_name: Option<String>,
        share_id_arg: &str,
        vault_name_arg: &str,
    ) -> Result<Self> {
        match (share_id, vault_name) {
            (Some(share_id), None) => Ok(Self::ShareId(ShareId::new(share_id))),
            (None, Some(vault_name)) => Ok(Self::VaultName(vault_name)),
            (None, None) => Err(anyhow!(
                "Please provide either {share_id_arg} or {vault_name_arg}"
            )),
            (Some(_), Some(_)) => Err(anyhow!(
                "Please provide either {share_id_arg} or {vault_name_arg}, not both"
            )),
        }
    }

    pub async fn share_id(&self, client: &PassClient) -> Result<ShareId> {
        match self {
            ShareQuery::ShareId(share_id) => Ok(share_id.clone()),
            ShareQuery::VaultName(name) => match client.find_vault(name).await {
                Ok(v) => Ok(v.share_id),
                Err(e) => Err(anyhow!("Error finding vault [{name}]: {e}")),
            },
        }
    }
}

pub enum ItemQuery {
    ItemId(ItemId),
    ItemTitle(String),
}

impl ItemQuery {
    pub fn new(item_id: Option<String>, item_title: Option<String>) -> Result<Self> {
        Self::new_with_arg_name(item_id, item_title, "--item-id", "--item-title")
    }

    pub fn new_with_arg_name(
        item_id: Option<String>,
        item_title: Option<String>,
        item_id_arg: &str,
        item_title_arg: &str,
    ) -> Result<Self> {
        match (item_id, item_title) {
            (Some(item_id), None) => Ok(Self::ItemId(ItemId::new(item_id))),
            (None, Some(item_title)) => Ok(Self::ItemTitle(item_title)),
            (None, None) => Err(anyhow!(
                "Please provide either {item_id_arg} or {item_title_arg}"
            )),
            (Some(_), Some(_)) => Err(anyhow!(
                "Please provide either {item_id_arg} or {item_title_arg}, not both"
            )),
        }
    }

    pub async fn item_id(&self, share_id: &ShareId, client: &PassClient) -> Result<ItemId> {
        match self {
            Self::ItemId(id) => Ok(id.clone()),
            Self::ItemTitle(title) => {
                let items = client
                    .list_items(share_id)
                    .await
                    .context("Error listing items")?;

                let matching_item = items
                    .iter()
                    .find(|item| item.content.title.eq(title))
                    .ok_or_else(|| anyhow!("No item found with title: {}", title))?;

                Ok(matching_item.id.clone())
            }
        }
    }
}
