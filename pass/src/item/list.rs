/*
 *  Copyright (c) 2026 Proton AG
 *  This file is part of Proton AG and Proton Pass.
 *
 *  Proton Pass is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  Proton Pass is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with Proton Pass.  If not, see <https://www.gnu.org/licenses/>.
 *
 */

use crate::domain::{FolderId, Item, ItemId, ShareId};
use crate::item::open::ItemWithItemKey;
use crate::item::page_cache::{
    FillState, ItemsForShareCache, ItemsForShareCacheType, ShareItems, ShareItemsHandle,
};
use crate::pagination::SincePagination;
use crate::{PassClient, PassClientContext};
use anyhow::{Context, Result};
use async_lock::Mutex;
use muon::GET;
use pass_derive::sdk_export;
use std::collections::HashSet;
use std::convert::Infallible;
use std::ops::ControlFlow;
use std::sync::Arc;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
struct GetItemsResponse {
    #[serde(rename = "Items")]
    pub items: ItemsList,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
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
    #[serde(rename = "CreateTime")]
    pub create_time: u64,
    #[serde(rename = "ModifyTime")]
    pub modify_time: u64,
    #[serde(rename = "FolderID")]
    pub folder_id: Option<String>,
}

#[sdk_export]
impl<C: PassClientContext> PassClient<C> {
    /// Lists the items available for a given ShareId
    #[sdk_export]
    pub async fn list_items(&self, share_id: &ShareId) -> Result<Vec<Item>> {
        let opened = match self
            .scan_items(share_id, false, |_| ControlFlow::<Infallible>::Continue(()))
            .await?
        {
            ControlFlow::Continue(opened) => opened,
            ControlFlow::Break(never) => match never {},
        };
        Ok(opened.into_iter().map(|i| i.item).collect())
    }

    /// Lists the items directly inside `folder_id` (not in its subfolders)
    /// of the vault behind `share_id`.
    #[sdk_export]
    pub async fn list_items_in_folder(
        &self,
        share_id: &ShareId,
        folder_id: &FolderId,
    ) -> Result<Vec<Item>> {
        let items = self
            .list_items(share_id)
            .await
            .context("Error listing items")?;
        Ok(items
            .into_iter()
            .filter(|item| item.folder_id.as_ref() == Some(folder_id))
            .collect())
    }

    #[sdk_export]
    pub async fn clear_items_cache(&self, share_id: &ShareId) {
        self.cache
            .update(ItemsForShareCacheType, |cache: &mut ItemsForShareCache| {
                cache.remove(share_id);
            })
            .await;
    }
}

impl<C: PassClientContext> PassClient<C> {
    // Visits the items of `share_id` until `visit` breaks, returning the break value.
    // Item pages are fetched and decrypted one at a time, so breaking skips the remaining
    // requests. The items cache for the share is populated as we visit pages. When the cache
    // is complete and nothing matched, the last page is polled again for newly created items.
    pub async fn list_items_until<B>(
        &self,
        share_id: &ShareId,
        mut visit: impl FnMut(&Item) -> ControlFlow<B>,
    ) -> Result<Option<B>> {
        match self
            .scan_items(share_id, true, |opened| visit(&opened.item))
            .await?
        {
            ControlFlow::Break(value) => Ok(Some(value)),
            ControlFlow::Continue(_) => Ok(None),
        }
    }

    /// Returns the first item of `share_id` matching `predicate`, without listing past it.
    pub async fn find_item_in_share(
        &self,
        share_id: &ShareId,
        mut predicate: impl FnMut(&Item) -> bool,
    ) -> Result<Option<Item>> {
        self.list_items_until(share_id, |item| {
            if predicate(item) {
                ControlFlow::Break(item.clone())
            } else {
                ControlFlow::Continue(())
            }
        })
        .await
    }

    async fn scan_items<B>(
        &self,
        share_id: &ShareId,
        poll_when_complete: bool,
        mut visit: impl FnMut(&ItemWithItemKey) -> ControlFlow<B>,
    ) -> Result<ControlFlow<B, Vec<ItemWithItemKey>>> {
        let handle = self.items_handle(share_id).await?;
        let mut guard = handle.lock().await;

        let cached = guard.items();
        for item in &cached {
            if let ControlFlow::Break(value) = visit(item) {
                return Ok(ControlFlow::Break(value));
            }
        }

        let mut pagination = match guard.state() {
            FillState::Complete(_) if !poll_when_complete => {
                return Ok(ControlFlow::Continue(cached));
            }
            state => state.cursor().clone(),
        };
        let mut known: HashSet<ItemId> = cached.iter().map(|i| i.item.id.clone()).collect();
        let mut opened = cached;
        loop {
            let page = self
                .fetch_items_page(share_id, &pagination)
                .await
                .context("Error fetching items")?;
            let page_size = page.revisions.len();
            trace!("Retrieved {page_size} items");
            if page_size == 0 {
                guard.set_state(FillState::Complete(pagination));
                break;
            }

            let opened_page: Vec<ItemWithItemKey> = self
                .open_items(share_id, page.revisions)
                .await
                .context("Error opening items")?
                .into_iter()
                .filter(|item| known.insert(item.item.id.clone()))
                .collect();

            let next_state = match pagination.clone().next(page.last_token) {
                Some(next) if page_size >= pagination.page_size => FillState::Partial(next),
                Some(next) => FillState::Complete(next),
                None => FillState::Complete(pagination.clone()),
            };

            for item in &opened_page {
                if let ControlFlow::Break(value) = visit(item) {
                    guard
                        .append(&opened_page, next_state)
                        .context("Error caching items")?;
                    return Ok(ControlFlow::Break(value));
                }
            }

            guard
                .append(&opened_page, next_state.clone())
                .context("Error caching items")?;
            opened.extend(opened_page);

            match next_state {
                FillState::Partial(next) => pagination = next,
                FillState::Complete(_) => break,
            }
        }

        trace!(
            "Finished item retrieval process. Retrieved {} items",
            opened.len()
        );
        Ok(ControlFlow::Continue(opened))
    }

    async fn items_handle(&self, share_id: &ShareId) -> Result<ShareItemsHandle> {
        let xor_key = self.memory_xor_key;
        let share_id = share_id.clone();
        self.cache
            .with_entry(
                ItemsForShareCacheType,
                ItemsForShareCache::new,
                |cache: &mut ItemsForShareCache| {
                    cache
                        .entry(share_id)
                        .or_insert_with(|| Arc::new(Mutex::new(ShareItems::new(xor_key))))
                        .clone()
                },
            )
            .await
            .context("Items cache entry has an unexpected type")
    }

    async fn fetch_items_page(
        &self,
        share_id: &ShareId,
        pagination: &SincePagination,
    ) -> Result<ItemsList> {
        let mut req = GET!("/pass/v1/share/{}/item", share_id)
            .query(("PageSize".to_string(), format!("{}", pagination.page_size)));
        if let Some(ref since) = pagination.since {
            req = req.query(("Since".to_string(), since.to_string()));
        }

        let res = self.send(req).await.context("Error fetching items page")?;
        let response: GetItemsResponse = assert_response!(res);
        Ok(response.items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::CodeResponse;
    use crate::domain::ItemState;
    use crate::test_tools::*;
    use muon_test::server::Request as RecordedRequest;
    use std::collections::VecDeque;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };

    fn setup_list_items_endpoint(
        server: &ProtonAPI,
        share_id: &str,
        revisions: Vec<ItemRevision>,
    ) -> Arc<AtomicBool> {
        server.handler_with_method(
            Method::GET,
            format!("/pass/v1/share/{}/item", share_id),
            move |_| {
                success(GetItemsResponse {
                    items: ItemsList {
                        revisions: revisions.clone(),
                        last_token: None,
                    },
                })
            },
        )
    }

    const PAGED_SHARE_ID: &str = "PagedShareID";
    const SECOND_PAGE_TOKEN: &str = "SecondPage";
    // The test server drops a handler once it has answered, so register a few copies
    const PAGED_HANDLER_COPIES: usize = 5;

    struct PagedItems {
        first_title: String,
        last_title: String,
        new_title: String,
        total: usize,
    }

    fn revision_for(item_id: String, encrypted: &EncryptItemContentsResult) -> ItemRevision {
        ItemRevisionBuilder::new(item_id)
            .with_content(crate::utils::b64_encode(&encrypted.encrypted_contents))
            .with_item_key(Some(crate::utils::b64_encode(
                &encrypted.encrypted_item_key,
            )))
            .with_state(ItemState::Active as u8)
            .build()
    }

    /// Serves a full first page whose first item is unique, then a second page with one more
    /// unique item. `fail_second_page` makes the second page fail while it is set, and
    /// `publish_new_item` adds a newly created item to the second page while it is set.
    fn setup_paged_items_endpoint(
        api: &ProtonAPI,
        fail_second_page: Arc<AtomicBool>,
        publish_new_item: Arc<AtomicBool>,
    ) -> PagedItems {
        let page_size = SincePagination::default().page_size;
        let first = create_random_item();
        let filler = encrypt_item_contents(create_random_item());
        let last = create_random_item();
        let new = create_random_item();

        let mut first_page = vec![revision_for(
            "First".to_string(),
            &encrypt_item_contents(first.clone()),
        )];
        first_page.extend((1..page_size).map(|i| revision_for(format!("Filler{i}"), &filler)));
        let second_page = vec![revision_for(
            "Last".to_string(),
            &encrypt_item_contents(last.clone()),
        )];
        let mut second_page_with_new = second_page.clone();
        second_page_with_new.push(revision_for(
            "New".to_string(),
            &encrypt_item_contents(new.clone()),
        ));

        for _ in 0..PAGED_HANDLER_COPIES {
            let first_page = first_page.clone();
            let second_page = second_page.clone();
            let second_page_with_new = second_page_with_new.clone();
            let fail_second_page = fail_second_page.clone();
            let publish_new_item = publish_new_item.clone();
            api.handler_with_method(
                Method::GET,
                format!("/pass/v1/share/{PAGED_SHARE_ID}/item"),
                move |req| {
                    let is_second_page = req
                        .uri()
                        .query()
                        .is_some_and(|q| q.contains(&format!("Since={SECOND_PAGE_TOKEN}")));
                    if !is_second_page {
                        return success(GetItemsResponse {
                            items: ItemsList {
                                revisions: first_page.clone(),
                                last_token: Some(SECOND_PAGE_TOKEN.to_string()),
                            },
                        });
                    }
                    if fail_second_page.load(Ordering::Relaxed) {
                        return response_from_status_and_payload(422, CodeResponse { code: 2001 });
                    }
                    let revisions = if publish_new_item.load(Ordering::Relaxed) {
                        second_page_with_new.clone()
                    } else {
                        second_page.clone()
                    };
                    success(GetItemsResponse {
                        items: ItemsList {
                            revisions,
                            last_token: None,
                        },
                    })
                },
            );
        }

        PagedItems {
            first_title: first.title,
            last_title: last.title,
            new_title: new.title,
            total: page_size + 1,
        }
    }

    fn no_flag() -> Arc<AtomicBool> {
        Arc::new(AtomicBool::new(false))
    }

    async fn paged_client(server: muon_test::Server) -> (TestPassClient, ProtonAPI) {
        let (raw_client, api) = server.client::<()>();
        let client = make_test_pass_client_with_setup(raw_client, &api, PlanType::Free).await;
        setup_vault_share(&api, PAGED_SHARE_ID);
        setup_share_keys(&api, PAGED_SHARE_ID);
        (client, api)
    }

    fn item_page_requests(requests: &VecDeque<RecordedRequest<Vec<u8>>>) -> usize {
        let path = format!("/pass/v1/share/{PAGED_SHARE_ID}/item");
        requests.iter().filter(|r| r.uri().path() == path).count()
    }

    #[muon_test::test]
    async fn test_list_items_empty_response(server: muon_test::Server) {
        let (raw_client, api) = server.client::<()>();
        const SHARE_ID: &str = "TestShareID1";

        let client = make_test_pass_client_with_setup(raw_client, &api, PlanType::Free).await;

        // Setup vault share and keys
        setup_vault_share(&api, SHARE_ID);
        setup_share_keys(&api, SHARE_ID);

        // Setup empty items list
        let handled = setup_list_items_endpoint(&api, SHARE_ID, vec![]);

        let recorder = api.new_recorder();
        let items = client
            .list_items(&share_id!(SHARE_ID))
            .await
            .expect("Should be able to list items");

        assert_hit!(handled);
        let requests = recorder.read();

        // Only request items. As it's empty, the share is not requested
        assert_eq!(requests.len(), 1);
        let req = requests.iter().next().unwrap();
        assert_eq!(
            format!("/pass/v1/share/{}/item", SHARE_ID),
            req.uri().path()
        );

        assert!(items.is_empty(), "Expected empty items list");
    }

    #[muon_test::test]
    async fn test_list_items_two_shares_with_one_item_each(server: muon_test::Server) {
        let (raw_client, api) = server.client::<()>();
        const SHARE_ID_1: &str = "TestShareID1";
        const SHARE_ID_2: &str = "TestShareID2";
        const ITEM_ID_1: &str = "Item1";
        const ITEM_ID_2: &str = "Item2";

        let client = make_test_pass_client_with_setup(raw_client, &api, PlanType::Free).await;

        // Setup first share
        setup_vault_share(&api, SHARE_ID_1);
        setup_share_keys(&api, SHARE_ID_1);

        // Setup second share
        setup_vault_share(&api, SHARE_ID_2);
        setup_share_keys(&api, SHARE_ID_2);

        // Create item data for first share
        let item_data_1 = create_random_item();
        let encrypted_data_1 = encrypt_item_contents(item_data_1.clone());
        let revision_1 = ItemRevisionBuilder::new(ITEM_ID_1.to_string())
            .with_content(crate::utils::b64_encode(
                &encrypted_data_1.encrypted_contents,
            ))
            .with_item_key(Some(crate::utils::b64_encode(
                &encrypted_data_1.encrypted_item_key,
            )))
            .with_state(ItemState::Active as u8)
            .build();

        // Create item data for second share
        let item_data_2 = create_random_item();
        let encrypted_data_2 = encrypt_item_contents(item_data_2.clone());
        let revision_2 = ItemRevisionBuilder::new(ITEM_ID_2.to_string())
            .with_content(crate::utils::b64_encode(
                &encrypted_data_2.encrypted_contents,
            ))
            .with_item_key(Some(crate::utils::b64_encode(
                &encrypted_data_2.encrypted_item_key,
            )))
            .with_state(ItemState::Active as u8)
            .build();

        // Setup endpoints
        let handled_1 = setup_list_items_endpoint(&api, SHARE_ID_1, vec![revision_1]);
        let handled_2 = setup_list_items_endpoint(&api, SHARE_ID_2, vec![revision_2]);

        // Fetch items for first share
        let items_1 = client
            .list_items(&share_id!(SHARE_ID_1))
            .await
            .expect("Should be able to list items for share 1");

        assert_hit!(handled_1);
        assert_eq!(1, items_1.len(), "Expected 1 item in share 1");
        assert_eq!(ITEM_ID_1, items_1[0].id.value());
        assert_eq!(item_data_1.title, items_1[0].content.title);

        // Fetch items for second share
        let items_2 = client
            .list_items(&share_id!(SHARE_ID_2))
            .await
            .expect("Should be able to list items for share 2");

        assert_hit!(handled_2);
        assert_eq!(1, items_2.len(), "Expected 1 item in share 2");
        assert_eq!(ITEM_ID_2, items_2[0].id.value());
        assert_eq!(item_data_2.title, items_2[0].content.title);
    }

    #[muon_test::test]
    async fn test_list_items_cache_is_used(server: muon_test::Server) {
        let (raw_client, api) = server.client::<()>();
        const SHARE_ID: &str = "TestShareID1";
        const ITEM_ID: &str = "Item1";

        let client = make_test_pass_client_with_setup(raw_client, &api, PlanType::Free).await;

        // Setup vault share and keys
        setup_vault_share(&api, SHARE_ID);
        setup_share_keys(&api, SHARE_ID);

        // Create item data
        let item_data = create_random_item();
        let encrypted_data = encrypt_item_contents(item_data.clone());
        let revision = ItemRevisionBuilder::new(ITEM_ID.to_string())
            .with_content(crate::utils::b64_encode(&encrypted_data.encrypted_contents))
            .with_item_key(Some(crate::utils::b64_encode(
                &encrypted_data.encrypted_item_key,
            )))
            .with_state(ItemState::Active as u8)
            .build();

        // Setup endpoint
        let handled = setup_list_items_endpoint(&api, SHARE_ID, vec![revision]);

        let recorder = api.new_recorder();

        // First fetch - should hit the endpoint
        let items_1 = client
            .list_items(&share_id!(SHARE_ID))
            .await
            .expect("Should be able to list items (first call)");

        assert_hit!(handled);
        assert_eq!(1, items_1.len());
        assert_eq!(ITEM_ID, items_1[0].id.value());

        let requests_after_first = recorder.read().len();
        assert!(requests_after_first > 0, "First call should make requests");

        // Second fetch - should use cache, no new requests
        let items_2 = client
            .list_items(&share_id!(SHARE_ID))
            .await
            .expect("Should be able to list items (second call)");

        let requests_after_second = recorder.read().len();
        assert_eq!(
            requests_after_first, requests_after_second,
            "Second call should not make new requests (cache should be used)"
        );

        // Verify the items are the same
        assert_eq!(1, items_2.len());
        assert_eq!(ITEM_ID, items_2[0].id.value());
        assert_eq!(item_data.title, items_2[0].content.title);
    }

    #[muon_test::test]
    async fn test_find_item_in_share_stops_at_first_page(server: muon_test::Server) {
        let (client, api) = paged_client(server).await;
        let paged = setup_paged_items_endpoint(&api, no_flag(), no_flag());
        let share_id = share_id!(PAGED_SHARE_ID);

        let recorder = api.new_recorder();
        let found = client
            .find_item_in_share(&share_id, |i| i.content.title == paged.first_title)
            .await
            .expect("Should be able to search items")
            .expect("Item should be found");

        assert_eq!("First", found.id.value());
        assert_eq!(1, item_page_requests(&recorder.read()));
    }

    #[muon_test::test]
    async fn test_list_items_until_break_caches_the_page_it_stopped_on(server: muon_test::Server) {
        let (client, api) = paged_client(server).await;
        let paged = setup_paged_items_endpoint(&api, no_flag(), no_flag());
        let share_id = share_id!(PAGED_SHARE_ID);

        let recorder = api.new_recorder();
        let mut visited = 0;
        let found = client
            .list_items_until(&share_id, |_| {
                visited += 1;
                ControlFlow::Break(visited)
            })
            .await
            .expect("Should be able to search items");
        assert_eq!(Some(1), found);
        assert_eq!(1, item_page_requests(&recorder.read()));

        let items = client
            .list_items(&share_id)
            .await
            .expect("Should be able to list items");

        assert_eq!(paged.total, items.len());
        assert_eq!(2, item_page_requests(&recorder.read()));
    }

    #[muon_test::test]
    async fn test_find_item_in_share_resumes_from_last_cached_page(server: muon_test::Server) {
        let (client, api) = paged_client(server).await;
        let paged = setup_paged_items_endpoint(&api, no_flag(), no_flag());
        let share_id = share_id!(PAGED_SHARE_ID);

        let recorder = api.new_recorder();
        let first = client
            .find_item_in_share(&share_id, |i| i.content.title == paged.first_title)
            .await
            .expect("Should be able to search items")
            .expect("Item should be found");
        assert_eq!("First", first.id.value());
        assert_eq!(1, item_page_requests(&recorder.read()));

        let repeated = client
            .find_item_in_share(&share_id, |i| i.content.title == paged.first_title)
            .await
            .expect("Should be able to search items")
            .expect("Item should be found in cache");
        assert_eq!("First", repeated.id.value());
        assert_eq!(1, item_page_requests(&recorder.read()));

        let missing = client
            .find_item_in_share(&share_id, |i| i.content.title == "missing")
            .await
            .expect("Should be able to search items");
        assert!(missing.is_none());
        assert_eq!(2, item_page_requests(&recorder.read()));

        let last = client
            .find_item_in_share(&share_id, |i| i.content.title == paged.last_title)
            .await
            .expect("Should be able to search items")
            .expect("Item should be found in cache");
        assert_eq!("Last", last.id.value());
        assert_eq!(2, item_page_requests(&recorder.read()));
    }

    #[muon_test::test]
    async fn test_find_item_in_share_polls_last_page_when_cache_is_complete(
        server: muon_test::Server,
    ) {
        let (client, api) = paged_client(server).await;
        let publish_new_item = no_flag();
        let paged = setup_paged_items_endpoint(&api, no_flag(), publish_new_item.clone());
        let share_id = share_id!(PAGED_SHARE_ID);

        let recorder = api.new_recorder();
        let items = client
            .list_items(&share_id)
            .await
            .expect("Should be able to list items");
        assert_eq!(paged.total, items.len());
        assert_eq!(2, item_page_requests(&recorder.read()));

        let missing = client
            .find_item_in_share(&share_id, |i| i.content.title == paged.new_title)
            .await
            .expect("Should be able to search items");
        assert!(missing.is_none());
        assert_eq!(3, item_page_requests(&recorder.read()));

        publish_new_item.store(true, Ordering::Relaxed);
        let found = client
            .find_item_in_share(&share_id, |i| i.content.title == paged.new_title)
            .await
            .expect("Should be able to search items")
            .expect("New item should be found by polling the last page");
        assert_eq!("New", found.id.value());
        assert_eq!(4, item_page_requests(&recorder.read()));

        let items = client
            .list_items(&share_id)
            .await
            .expect("Should be able to list items");
        assert_eq!(paged.total + 1, items.len());
        assert_eq!(4, item_page_requests(&recorder.read()));
    }

    #[muon_test::test]
    async fn test_failed_page_keeps_earlier_pages_and_retries_same_page(server: muon_test::Server) {
        let (client, api) = paged_client(server).await;
        let fail_second_page = Arc::new(AtomicBool::new(true));
        let paged = setup_paged_items_endpoint(&api, fail_second_page.clone(), no_flag());
        let share_id = share_id!(PAGED_SHARE_ID);

        client
            .list_items(&share_id)
            .await
            .expect_err("Second page failure should fail the listing");

        fail_second_page.store(false, Ordering::Relaxed);
        let recorder = api.new_recorder();
        let items = client
            .list_items(&share_id)
            .await
            .expect("Should be able to list items");

        assert_eq!(paged.total, items.len());
        assert_eq!(1, item_page_requests(&recorder.read()));
    }

    #[muon_test::test]
    async fn test_failed_page_keeps_cached_items_lookupable(server: muon_test::Server) {
        let (client, api) = paged_client(server).await;
        let fail_second_page = Arc::new(AtomicBool::new(true));
        let paged = setup_paged_items_endpoint(&api, fail_second_page.clone(), no_flag());
        let share_id = share_id!(PAGED_SHARE_ID);

        client
            .find_item_in_share(&share_id, |i| i.content.title == "missing")
            .await
            .expect_err("Second page failure should fail the search");

        let recorder = api.new_recorder();
        let found = client
            .find_item_in_share(&share_id, |i| i.content.title == paged.first_title)
            .await
            .expect("Should be able to search cached items")
            .expect("First page item should still be cached");

        assert_eq!("First", found.id.value());
        assert_eq!(0, item_page_requests(&recorder.read()));
    }

    #[muon_test::test]
    async fn test_clear_items_cache_resets_to_first_page(server: muon_test::Server) {
        let (client, api) = paged_client(server).await;
        let paged = setup_paged_items_endpoint(&api, no_flag(), no_flag());
        let share_id = share_id!(PAGED_SHARE_ID);

        let recorder = api.new_recorder();
        client
            .find_item_in_share(&share_id, |i| i.content.title == paged.first_title)
            .await
            .expect("Should be able to search items")
            .expect("Item should be found");
        assert_eq!(1, item_page_requests(&recorder.read()));

        client.clear_items_cache(&share_id).await;

        client
            .find_item_in_share(&share_id, |i| i.content.title == paged.first_title)
            .await
            .expect("Should be able to search items")
            .expect("Item should be found");
        assert_eq!(2, item_page_requests(&recorder.read()));
    }
}
