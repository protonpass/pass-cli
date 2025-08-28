use crate::PassClient;
use crate::pagination::Pagination;
use crate::share::{EncryptedShareKey, ShareKey, ShareKeys};
use anyhow::{Context, Result};
use async_lock::{Mutex, RwLock};
use muon::GET;
use pass_domain::ShareId;
use std::collections::HashMap;
use std::sync::Arc;

/// Per-ShareId cache with efficient locking to prevent duplicate fetches
#[derive(Clone)]
struct ShareKeyCache {
    // The actual cache data
    cache: Arc<RwLock<HashMap<ShareId, ShareKeys>>>,
    // Per-ShareId locks to prevent duplicate fetches
    locks: Arc<Mutex<HashMap<ShareId, Arc<Mutex<()>>>>>,
}

impl ShareKeyCache {
    fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            locks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Get share keys with per-ShareId locking to prevent duplicate fetches
    async fn get_or_fetch<F, Fut>(&self, share_id: &ShareId, fetch_fn: F) -> Result<ShareKeys>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<ShareKeys>>,
    {
        let share_id = share_id.clone();

        // Fast path: check cache with read lock
        {
            let cache = self.cache.read().await;
            if let Some(keys) = cache.get(&share_id) {
                trace!("Returning cached share keys for {}", share_id);
                return Ok(keys.clone());
            }
        }

        // Slow path: get per-ShareId lock to prevent duplicate fetches
        let share_lock = {
            let mut locks = self.locks.lock().await;
            locks
                .entry(share_id.clone())
                .or_insert_with(|| Arc::new(Mutex::new(())))
                .clone()
        };

        let _guard = share_lock.lock().await;

        // Double-check: another thread might have populated the cache
        {
            let cache = self.cache.read().await;
            if let Some(keys) = cache.get(&share_id) {
                trace!(
                    "Returning cached share keys after double-check for {}",
                    share_id
                );
                return Ok(keys.clone());
            }
        }

        // We have the lock and no cached value - fetch it
        trace!("Fetching share keys for {}", share_id);
        let keys = fetch_fn().await?;

        // Store in cache
        {
            let mut cache = self.cache.write().await;
            cache.insert(share_id.clone(), keys.clone());
        }

        Ok(keys)
    }
}

// Cache type for the ShareKeyCache instance
struct ShareKeyCacheType;

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
        // Ensure the ShareKeyCache exists in the client cache
        self.cache
            .ensure_has_value(ShareKeyCacheType, ShareKeyCache::new)
            .await;

        // Get the ShareKeyCache from the client cache
        let share_key_cache: ShareKeyCache = self
            .cache
            .get(ShareKeyCacheType)
            .await
            .expect("ShareKeyCache should exist after ensure_has_value");

        let client = self.clone();
        let share_id_for_fetch = share_id.clone();

        share_key_cache
            .get_or_fetch(share_id, || async move {
                let keys_responses = client
                    .get_share_keys_paginated(&share_id_for_fetch)
                    .await
                    .context("Error requesting share keys")?;

                let mut keys = vec![];
                for key in keys_responses {
                    let decoded = crate::utils::b64_decode(&key.key)
                        .context("Error decoding ShareKey key")?;
                    keys.push(ShareKey::new(key.key_rotation, EncryptedShareKey(decoded)));
                }

                Ok(ShareKeys::new(keys))
            })
            .await
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
        trace!(
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
        trace!("Retrieving page {:?}", pagination);
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
        trace!("Retrieved {} share keys", share_key_count);

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
