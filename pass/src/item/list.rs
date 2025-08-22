use crate::PassClient;
use crate::item::open::ItemWithItemKey;
use crate::pagination::SincePagination;
use crate::utils::debug_response;
use anyhow::{Context, Result, anyhow};
use muon::GET;
use pass_domain::{Item, ShareId};
use std::collections::HashMap;

struct ItemsForShareCacheType;
type ItemsForShareCache = HashMap<ShareId, Vec<ItemWithItemKey>>;

#[derive(Clone, Debug, serde::Deserialize)]
struct GetItemsResponse {
    #[serde(rename = "Items")]
    pub items: ItemsList,
}

#[derive(Clone, Debug, serde::Deserialize)]
struct ItemsList {
    #[serde(rename = "RevisionsData")]
    pub revisions: Vec<ItemRevision>,
    #[serde(rename = "LastToken")]
    pub last_token: Option<String>,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[allow(dead_code)]
pub(crate) struct ItemRevision {
    #[serde(rename = "ItemID")]
    pub item_id: String,
    #[serde(rename = "Revision")]
    pub revision: u64,
    #[serde(rename = "ContentFormatVersion")]
    pub content_format_version: i32,
    #[serde(rename = "KeyRotation")]
    pub key_rotation: u8,
    #[serde(rename = "Content")]
    pub content: String,
    #[serde(rename = "ItemKey")]
    pub item_key: Option<String>,
    #[serde(rename = "State")]
    pub state: u8,
    #[serde(rename = "Flags")]
    pub flags: u64,
    #[serde(rename = "AliasEmail")]
    pub alias_email: Option<String>,
}

impl PassClient {
    pub async fn list_items(&self, share_id: &ShareId) -> Result<Vec<Item>> {
        {
            let share_id = share_id.clone();
            self.cache
                .ensure_has_value(ItemsForShareCacheType, ItemsForShareCache::new)
                .await;

            let cached: Option<ItemsForShareCache> = self.cache.get(ItemsForShareCacheType).await;
            if let Some(cached) = cached {
                let share_items = cached.get(&share_id);
                if let Some(cached_items) = share_items {
                    let items = cached_items.iter().map(|i| &i.item).cloned().collect();
                    trace!("Returning cached items for share {share_id}");
                    return Ok(items);
                }
            }
        }

        let items = self
            .fetch_items(share_id)
            .await
            .context("Error fetching items")?;

        let opened = self
            .open_items(share_id, items)
            .await
            .context("Error opening items")?;

        self.cache
            .update(
                ItemsForShareCacheType,
                |share_items: &mut ItemsForShareCache| {
                    share_items.insert(share_id.clone(), opened.clone());
                },
            )
            .await;

        let items = opened.into_iter().map(|i| i.item).collect();
        Ok(items)
    }

    async fn fetch_items(&self, share_id: &ShareId) -> Result<Vec<ItemRevision>> {
        let mut items = Vec::new();
        let mut pagination = SincePagination::default();
        loop {
            let mut req = GET!("/pass/v1/share/{}/item", share_id)
                .query(("PageSize".to_string(), format!("{}", pagination.page_size)));

            if let Some(ref since) = pagination.since {
                req = req.query(("SinceToken".to_string(), since.to_string()));
            }

            let res = self
                .client
                .send(req)
                .await
                .context("Error fetching items page")?;

            if !res.status().is_success() {
                debug_response(&res);
                return Err(anyhow!("Error fetching items"));
            }

            let response: GetItemsResponse = res.body_json().context("Unable to parse response")?;
            let response_content = response.items;
            let revisions = response_content.revisions;

            trace!("Retrieved {} items", revisions.len());
            if !revisions.is_empty() {
                let retrieved_size = revisions.len();
                items.extend(revisions);
                if retrieved_size < pagination.page_size {
                    break;
                }

                match pagination.next(response_content.last_token) {
                    Some(p) => pagination = p,
                    None => break,
                }
            } else {
                break;
            }
        }

        trace!(
            "Finished item retrieval process. Retrieved {} items",
            items.len()
        );

        Ok(items)
    }

    pub(crate) async fn clear_items_cache(&self, share_id: &ShareId) {
        self.cache
            .update(ItemsForShareCacheType, |cache: &mut ItemsForShareCache| {
                cache.remove(share_id);
            })
            .await;
    }
}
