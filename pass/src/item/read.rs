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

use crate::common::CodeResponse;
use crate::domain::{ItemId, ShareId};
use crate::{PassClient, PassClientContext, PlanType};
use anyhow::{Context, Result};
use muon::PUT;
use muon::common::RetryPolicy;
use std::collections::HashSet;

/// Maximum number of items reported in a single request.
const ITEM_READ_BATCH_SIZE: usize = 100;

struct ItemReadReportedCacheType;

type ItemReadReported = HashSet<(ShareId, ItemId)>;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct ItemTimesRequest {
    #[serde(rename = "ItemTimes")]
    pub item_times: Vec<ItemReadRequest>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct ItemReadRequest {
    #[serde(rename = "ItemID")]
    item_id: String,
    #[serde(rename = "Timestamp")]
    timestamp: i64,
}

impl<C: PassClientContext> PassClient<C> {
    pub async fn mark_item_read(&self, share_id: &ShareId, item_id: &ItemId) {
        self.mark_items_read(share_id, std::slice::from_ref(item_id))
            .await
    }

    pub async fn mark_items_read(&self, share_id: &ShareId, item_ids: &[ItemId]) {
        if item_ids.is_empty() {
            return;
        }
        match self.get_plan_type().await {
            Ok(PlanType::Business) => {}
            Ok(plan) => {
                debug!("Plan is {plan:?}, not reporting item read");
                return;
            }
            Err(e) => {
                warn!("Could not determine the plan type, not reporting item read: {e:?}");
                return;
            }
        }

        let claimed = self.claim_unreported(share_id, item_ids).await;
        for batch in claimed.chunks(ITEM_READ_BATCH_SIZE) {
            match self.send_items_read(share_id, batch).await {
                Ok(code) => {
                    debug!(
                        "Reported items read: share={share_id} count={} code={code}",
                        batch.len()
                    )
                }
                Err(e) => {
                    warn!(
                        "Failed to report items read: share={share_id} count={}: {e:?}",
                        batch.len()
                    );
                    self.unclaim(share_id, batch).await;
                }
            }
        }
    }

    async fn claim_unreported(&self, share_id: &ShareId, item_ids: &[ItemId]) -> Vec<ItemId> {
        self.cache
            .with_entry(
                ItemReadReportedCacheType,
                ItemReadReported::new,
                |reported: &mut ItemReadReported| {
                    item_ids
                        .iter()
                        .filter(|item_id| reported.insert((share_id.clone(), (*item_id).clone())))
                        .cloned()
                        .collect()
                },
            )
            .await
            .unwrap_or_default()
    }

    async fn unclaim(&self, share_id: &ShareId, item_ids: &[ItemId]) {
        self.cache
            .update(
                ItemReadReportedCacheType,
                |reported: &mut ItemReadReported| {
                    for item_id in item_ids {
                        reported.remove(&(share_id.clone(), item_id.clone()));
                    }
                },
            )
            .await;
    }

    async fn send_items_read(&self, share_id: &ShareId, item_ids: &[ItemId]) -> Result<u32> {
        let now = jiff::Timestamp::now().as_second();
        let body = ItemTimesRequest {
            item_times: item_ids
                .iter()
                .map(|id| ItemReadRequest {
                    item_id: id.to_string(),
                    timestamp: now,
                })
                .collect(),
        };
        let req = PUT!("/pass/v1/share/{share_id}/item/read")
            .body_json(body)
            .context("Error creating item read request")?
            // Best effort reporting
            .retry_policy(RetryPolicy::default().never())
            .allowed_time(std::time::Duration::from_secs(5));
        let res = self
            .send(req)
            .await
            .context("Error sending item read request")?;
        let response: CodeResponse = assert_response!(res);
        response.success_guard()?;
        Ok(response.code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::CodeResponse;
    use crate::test_tools::*;
    use muon_test::server::Request;

    const PUT_PATH: &str = "/pass/v1/share/share1/item/read";
    const ACCESS_PATH: &str = "/pass/v1/user/access";
    // The test server drops a handler once it has answered, so register a few copies
    const HANDLER_COPIES: usize = 5;

    fn ids() -> (ShareId, ItemId) {
        (
            ShareId::new("share1".to_string()),
            ItemId::new("item1".to_string()),
        )
    }

    fn put_handlers(api: &ProtonAPI, code: u32, status: u16) {
        for _ in 0..HANDLER_COPIES {
            api.handler_with_method(Method::PUT, PUT_PATH, move |_| {
                response_from_status_and_payload(status, CodeResponse { code })
            });
        }
    }

    fn access_handlers(api: &ProtonAPI, plan: PlanType) {
        for _ in 0..HANDLER_COPIES {
            setup_user_access(api, plan);
        }
    }

    fn count(requests: Vec<Request<Vec<u8>>>, method: &str, path: &str) -> usize {
        requests
            .iter()
            .filter(|r| r.method().as_str() == method && r.uri().path() == path)
            .count()
    }

    async fn client_with_plan(
        server: muon_test::Server,
        plan: PlanType,
    ) -> (TestPassClient, ProtonAPI) {
        let (raw_client, api) = server.client::<()>();
        let client = make_test_pass_client(raw_client, &api).await;
        access_handlers(&api, plan);
        (client, api)
    }

    #[muon_test::test]
    async fn business_sends_one_put_with_expected_body(server: muon_test::Server) {
        let (client, api) = client_with_plan(server, PlanType::Business).await;
        put_handlers(&api, 1000, 200);
        let recorder = api.new_recorder();
        let (share, item) = ids();

        let before = jiff::Timestamp::now().as_second();
        client.mark_item_read(&share, &item).await;
        let after = jiff::Timestamp::now().as_second();

        assert_eq!(
            1,
            count(recorder.read().into_iter().collect(), "PUT", PUT_PATH)
        );
        let put = recorder
            .read()
            .into_iter()
            .find(|r| r.method().as_str() == "PUT")
            .unwrap();
        let body: ItemTimesRequest = serde_json::from_slice(put.body()).unwrap();
        assert_eq!(1, body.item_times.len());
        assert_eq!("item1", body.item_times[0].item_id);
        let ts = body.item_times[0].timestamp;
        assert!(ts >= before - 5 && ts <= after + 5);
    }

    #[muon_test::test]
    async fn marking_same_item_twice_sends_one_put(server: muon_test::Server) {
        let (client, api) = client_with_plan(server, PlanType::Business).await;
        put_handlers(&api, 1000, 200);
        let recorder = api.new_recorder();
        let (share, item) = ids();

        client.mark_item_read(&share, &item).await;
        client.mark_item_read(&share, &item).await;
        assert_eq!(
            1,
            count(recorder.read().into_iter().collect(), "PUT", PUT_PATH)
        );
    }

    #[muon_test::test]
    async fn duplicates_in_slice_send_one_put(server: muon_test::Server) {
        let (client, api) = client_with_plan(server, PlanType::Business).await;
        put_handlers(&api, 1000, 200);
        let recorder = api.new_recorder();
        let (share, item) = ids();

        client.mark_items_read(&share, &[item.clone(), item]).await;
        assert_eq!(
            1,
            count(recorder.read().into_iter().collect(), "PUT", PUT_PATH)
        );
    }

    #[muon_test::test]
    async fn items_are_sent_in_batches_of_100(server: muon_test::Server) {
        let (client, api) = client_with_plan(server, PlanType::Business).await;
        put_handlers(&api, 1000, 200);
        let recorder = api.new_recorder();
        let share = ShareId::new("share1".to_string());
        let items: Vec<ItemId> = (0..250).map(|i| ItemId::new(format!("item{i}"))).collect();

        client.mark_items_read(&share, &items).await;

        let sizes: Vec<usize> = recorder
            .read()
            .into_iter()
            .filter(|r| r.method().as_str() == "PUT" && r.uri().path() == PUT_PATH)
            .map(|r| {
                let body: ItemTimesRequest = serde_json::from_slice(r.body()).unwrap();
                body.item_times.len()
            })
            .collect();
        assert_eq!(vec![100, 100, 50], sizes);
    }

    #[muon_test::test]
    async fn failed_put_is_swallowed_and_retried(server: muon_test::Server) {
        let (client, api) = client_with_plan(server, PlanType::Business).await;
        put_handlers(&api, 2011, 400);
        let recorder = api.new_recorder();
        let (share, item) = ids();

        client.mark_item_read(&share, &item).await;
        client.mark_item_read(&share, &item).await;
        assert_eq!(
            2,
            count(recorder.read().into_iter().collect(), "PUT", PUT_PATH)
        );
    }

    #[muon_test::test]
    async fn server_error_is_not_retried_within_a_single_mark(server: muon_test::Server) {
        let (client, api) = client_with_plan(server, PlanType::Business).await;
        put_handlers(&api, 2011, 503);
        let recorder = api.new_recorder();
        let (share, item) = ids();

        let start = std::time::Instant::now();
        client.mark_item_read(&share, &item).await;
        client.mark_item_read(&share, &item).await;
        // One attempt per mark: muon's default retry/backoff must not apply.
        assert_eq!(
            2,
            count(recorder.read().into_iter().collect(), "PUT", PUT_PATH)
        );
        assert!(start.elapsed() < std::time::Duration::from_secs(5));
    }

    #[muon_test::test]
    async fn non_success_code_is_retried(server: muon_test::Server) {
        let (client, api) = client_with_plan(server, PlanType::Business).await;
        put_handlers(&api, 2011, 200);
        let recorder = api.new_recorder();
        let (share, item) = ids();

        client.mark_item_read(&share, &item).await;
        client.mark_item_read(&share, &item).await;
        assert_eq!(
            2,
            count(recorder.read().into_iter().collect(), "PUT", PUT_PATH)
        );
    }

    async fn assert_no_put_for(server: muon_test::Server, plan: PlanType) {
        let (client, api) = client_with_plan(server, plan).await;
        put_handlers(&api, 1000, 200);
        let recorder = api.new_recorder();
        let (share, item) = ids();

        client.mark_item_read(&share, &item).await;
        assert_eq!(
            0,
            count(recorder.read().into_iter().collect(), "PUT", PUT_PATH)
        );
    }

    #[muon_test::test]
    async fn free_sends_nothing(server: muon_test::Server) {
        assert_no_put_for(server, PlanType::Free).await;
    }

    #[muon_test::test]
    async fn plus_sends_nothing(server: muon_test::Server) {
        assert_no_put_for(server, PlanType::Plus).await;
    }

    #[muon_test::test]
    async fn plan_lookup_failure_sends_nothing_and_is_not_cached(server: muon_test::Server) {
        let (raw_client, api) = server.client::<()>();
        let client = make_test_pass_client(raw_client, &api).await;
        for _ in 0..HANDLER_COPIES {
            api.handler(ACCESS_PATH, |_| {
                response_from_status_and_payload(400, CodeResponse { code: 2011 })
            });
        }
        put_handlers(&api, 1000, 200);
        let recorder = api.new_recorder();
        let (share, item) = ids();

        client.mark_item_read(&share, &item).await;
        client.mark_item_read(&share, &item).await;
        assert_eq!(
            0,
            count(recorder.read().into_iter().collect(), "PUT", PUT_PATH)
        );
        assert_eq!(
            2,
            count(recorder.read().into_iter().collect(), "GET", ACCESS_PATH)
        );
    }

    #[muon_test::test]
    async fn plan_type_is_memoised(server: muon_test::Server) {
        let (client, api) = client_with_plan(server, PlanType::Business).await;
        put_handlers(&api, 1000, 200);
        let recorder = api.new_recorder();
        let share = ShareId::new("share1".into());

        client
            .mark_item_read(&share, &ItemId::new("a".into()))
            .await;
        client
            .mark_item_read(&share, &ItemId::new("b".into()))
            .await;
        assert_eq!(
            1,
            count(recorder.read().into_iter().collect(), "GET", ACCESS_PATH)
        );
    }

    #[muon_test::test]
    async fn prior_user_access_avoids_extra_lookup(server: muon_test::Server) {
        let (client, api) = client_with_plan(server, PlanType::Business).await;
        put_handlers(&api, 1000, 200);
        client.get_user_access().await.unwrap();
        let recorder = api.new_recorder();
        let (share, item) = ids();

        client.mark_item_read(&share, &item).await;
        assert_eq!(
            0,
            count(recorder.read().into_iter().collect(), "GET", ACCESS_PATH)
        );
        assert_eq!(
            1,
            count(recorder.read().into_iter().collect(), "PUT", PUT_PATH)
        );
    }
}
