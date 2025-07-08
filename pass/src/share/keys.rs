use crate::PassClient;
use crate::pagination::Pagination;
use crate::share::{EncryptedShareKey, ShareKey, ShareKeys};
use anyhow::{Context, Result};
use muon::GET;
use pass_domain::ShareId;
use std::collections::HashMap;

struct ShareKeysCacheType;
type ShareKeysCache = HashMap<ShareId, ShareKeys>;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct GetShareKeysResponse {
    #[serde(rename = "ShareKeys")]
    pub keys: ShareKeyList,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct ShareKeyList {
    #[serde(rename = "Keys")]
    pub keys: Vec<ShareKeyResponse>,
    #[serde(rename = "Total")]
    pub total: u32,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct ShareKeyResponse {
    #[serde(rename = "KeyRotation")]
    pub key_rotation: u8,
    #[serde(rename = "Key")]
    pub key: String,
    #[serde(rename = "CreateTime")]
    pub create_time: i64,
}

impl PassClient {
    pub(crate) async fn get_share_keys(&self, share_id: &ShareId) -> Result<ShareKeys> {
        {
            let share_id = share_id.clone();
            self.cache
                .ensure_has_value(ShareKeysCacheType, ShareKeysCache::new)
                .await;

            let cached: Option<ShareKeysCache> = self.cache.get(ShareKeysCacheType).await;
            if let Some(cached) = cached {
                let share_keys = cached.get(&share_id);
                if let Some(cached_share_keys) = share_keys {
                    println!(">>> Returning cached share keys");
                    return Ok(cached_share_keys.clone());
                }
            }
        }

        let keys_responses = self
            .get_share_keys_paginated(share_id)
            .await
            .context("Error requesting share keys")?;

        let mut keys = vec![];
        for key in keys_responses {
            let decoded =
                crate::utils::b64_decode(&key.key).context("Error decoding ShareKey key")?;
            keys.push(ShareKey::new(key.key_rotation, EncryptedShareKey(decoded)));
        }

        let keys = ShareKeys::new(keys);
        self.cache
            .update(ShareKeysCacheType, |keys_cache: &mut ShareKeysCache| {
                keys_cache.insert(share_id.clone(), keys.clone());
            })
            .await;

        Ok(keys)
    }

    async fn get_share_keys_paginated(&self, share_id: &ShareId) -> Result<Vec<ShareKeyResponse>> {
        let mut share_keys = vec![];
        let mut pagination = Pagination::default_paginated();
        loop {
            let keep_looping = self
                .request_share_key_page(share_id, &mut share_keys, &pagination)
                .await?;
            if keep_looping {
                pagination = pagination.next();
            } else {
                break;
            }
        }
        debug!(
            "Finished ShareKey retrieval process. Retrieved {} keys",
            share_keys.len()
        );

        Ok(share_keys)
    }

    async fn request_share_key_page(
        &self,
        share_id: &ShareId,
        share_keys: &mut Vec<ShareKeyResponse>,
        pagination: &Pagination,
    ) -> Result<bool> {
        debug!("Retrieving page {:?}", pagination);
        let req = GET!("/pass/v1/share/{}/key", share_id.value())
            .query(("Page", format!("{}", pagination.page())))
            .query(("PageSize", format!("{}", pagination.page_size())));
        let res = self
            .client
            .send(req)
            .await
            .context("Error getting page for share keys")?;

        let page_keys: GetShareKeysResponse =
            res.body_json().context("Error decoding share keys page")?;

        let share_key_count = page_keys.keys.keys.len();
        debug!("Retrieved {} items", share_key_count);

        // Always store the keys
        share_keys.extend(page_keys.keys.keys);

        // Check item keys, as we can have more item keys than vault keys (case of Item share)
        if share_key_count < pagination.page_size() {
            Ok(false)
        } else {
            Ok(true)
        }
    }
}
