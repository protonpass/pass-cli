use crate::item::list::ItemRevision;
use crate::pagination::SincePagination;
use crate::{PassClient, PassClientContext};
use anyhow::{Context, Result};
use muon::GET;
use pass_domain::{ItemId, ShareId};
use std::collections::HashMap;

#[derive(Debug, serde::Deserialize)]
struct GetItemRevisionListResponse {
    #[serde(rename = "Revisions")]
    pub revisions: ItemRevisionListResponse,
}

#[derive(Debug, serde::Deserialize)]
struct ItemRevisionListResponse {
    #[serde(rename = "RevisionsData")]
    pub revisions_data: Vec<ItemRevision>,
    #[serde(rename = "Total")]
    pub total: u32,
    #[serde(rename = "LastToken")]
    pub last_token: Option<String>,
}

struct ItemRevisionsCacheType;

#[derive(Clone, Hash, Eq, PartialEq)]
struct ItemRevisionsCacheKey {
    share_id: ShareId,
    item_id: ItemId,
}

impl ItemRevisionsCacheKey {
    fn new(share_id: &ShareId, item_id: &ItemId) -> Self {
        Self {
            share_id: share_id.clone(),
            item_id: item_id.clone(),
        }
    }
}

type ItemRevisionsForShareItemCache = HashMap<ItemRevisionsCacheKey, Vec<ItemRevision>>;

impl<C: PassClientContext> PassClient<C> {
    pub(crate) async fn get_item_revisions(
        &self,
        share_id: &ShareId,
        item_id: &ItemId,
    ) -> Result<Vec<ItemRevision>> {
        {
            self.cache
                .ensure_has_value(ItemRevisionsCacheType, ItemRevisionsForShareItemCache::new)
                .await;

            let cached: Option<ItemRevisionsForShareItemCache> =
                self.cache.get(ItemRevisionsCacheType).await;
            if let Some(cached) = cached {
                let key = ItemRevisionsCacheKey::new(share_id, item_id);
                let revisions: Option<&Vec<ItemRevision>> = cached.get(&key);
                if let Some(cached_items) = revisions {
                    return Ok(cached_items.clone().to_vec());
                }
            }
        }

        let revisions = self
            .fetch_item_revisions(share_id, item_id)
            .await
            .context("Error fetching item revisions")?;
        let cache_key = ItemRevisionsCacheKey::new(share_id, item_id);

        let revisions_clone = revisions.clone();
        self.cache
            .update(
                ItemRevisionsCacheType,
                |item_revisions: &mut ItemRevisionsForShareItemCache| {
                    item_revisions.insert(cache_key, revisions_clone);
                },
            )
            .await;
        Ok(revisions)
    }

    async fn fetch_item_revisions(
        &self,
        share_id: &ShareId,
        item_id: &ItemId,
    ) -> Result<Vec<ItemRevision>> {
        let mut revisions_accumulated = Vec::new();
        let mut pagination = SincePagination::default();
        loop {
            let mut req = GET!("/pass/v1/share/{share_id}/item/{item_id}/revision")
                .query(("PageSize".to_string(), format!("{}", pagination.page_size)));

            if let Some(ref since) = pagination.since {
                req = req.query(("Since".to_string(), since.to_string()));
            }

            let res = self.send(req).await.context("Error fetching items page")?;

            let response: GetItemRevisionListResponse = assert_response!(res);
            let response_content = response.revisions;
            let page_revisions = response_content.revisions_data;

            trace!("Retrieved {} revisions", page_revisions.len());
            if !page_revisions.is_empty() {
                let retrieved_size = page_revisions.len();
                revisions_accumulated.extend(page_revisions);
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
            "Finished item revision retrieval process. Retrieved {} revisions",
            revisions_accumulated.len()
        );

        Ok(revisions_accumulated)
    }
}
