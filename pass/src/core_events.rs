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

use crate::{PassClient, PassClientContext};
use anyhow::Result;
use muon::GET;
use pass_domain::AccountType;

const CORE_EVENTS_SYNC_INTERVAL_SECS: i64 = 30 * 60; // 30 mins

#[derive(serde::Deserialize)]
struct LatestEventResponse {
    #[serde(rename = "EventID")]
    event_id: String,
}

#[derive(serde::Deserialize)]
struct CoreEventsResponse {
    #[serde(rename = "EventID")]
    event_id: String,
    #[serde(rename = "More")]
    more: u8,
    #[serde(rename = "User")]
    user: Option<CoreUserEvent>,
}

#[derive(serde::Deserialize)]
struct CoreUserEvent {
    #[serde(rename = "Keys", default)]
    keys: Vec<serde_json::Value>,
}

/// Called once at CLI bootstrap (after session load, before commands dispatch).
/// Checks for key changes since last run and, if found, clears the key cache so
/// the next `get_user_keys()` call re-fetches from the API.
/// No-ops for PAT and agent sessions.
pub async fn bootstrap_event_sync<C: PassClientContext>(client: &PassClient<C>) {
    if client.account_type() == AccountType::PersonalAccessToken
        || client.account_type() == AccountType::AgentSession
    {
        return;
    }

    if let Err(e) = sync_core_events(client).await {
        warn!("Failed to sync core events during bootstrap: {e:#}");
    }
}

async fn sync_core_events<C: PassClientContext>(client: &PassClient<C>) -> Result<()> {
    let storage = client
        .client_features
        .get_data_storage()
        .await?
        .get_core_event_storage()
        .await;

    let mut current_id = match storage.get_cursor().await? {
        None => {
            debug!("No core event cursor stored, fetching latest event ID");
            let res = client.send(GET!("/core/v4/events/latest")).await?;
            let response: LatestEventResponse = assert_response!(res);
            debug!(
                "Seeding core event cursor with event_id={}",
                response.event_id
            );
            storage.set_cursor(&response.event_id).await?;
            return Ok(());
        }
        Some(entry) => {
            let age_secs = jiff::Timestamp::now().as_second() - entry.updated_at;
            if age_secs < CORE_EVENTS_SYNC_INTERVAL_SECS {
                debug!("Core event cursor is {age_secs}s old, skipping sync");
                return Ok(());
            }
            entry.event_id
        }
    };

    let mut keys_changed = false;
    debug!("Fetching core events since event_id={current_id}");
    loop {
        let res = client.send(GET!("/core/v4/events/{current_id}")).await?;
        let response: CoreEventsResponse = assert_response!(res);

        if let Some(ref user) = response.user
            && !user.keys.is_empty()
        {
            debug!("Core event contains key changes, will refresh user keys");
            keys_changed = true;
        }

        current_id = response.event_id;

        if response.more == 0 {
            break;
        }
        debug!("More core events pending, fetching next page from event_id={current_id}");
    }

    if keys_changed {
        debug!("User keys changed during bootstrap, invalidating key cache");
        client.clear_user_keys_cache().await;
    }
    debug!("Writing core event cursor {current_id}");
    storage.set_cursor(&current_id).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_tools::*;

    async fn read_cursor<C: PassClientContext>(client: &PassClient<C>) -> Result<Option<String>> {
        let storage = client
            .client_features
            .get_data_storage()
            .await?
            .get_core_event_storage()
            .await;
        Ok(storage.get_cursor().await?.map(|e| e.event_id))
    }

    async fn write_cursor<C: PassClientContext>(
        client: &PassClient<C>,
        event_id: &str,
    ) -> Result<()> {
        let storage = client
            .client_features
            .get_data_storage()
            .await?
            .get_core_event_storage()
            .await;
        storage.set_cursor(event_id).await
    }

    #[muon_test::test]
    async fn first_run_seeds_cursor_without_key_refresh(server: muon_test::Server) {
        let (raw_client, api) = server.client::<()>();
        let client = make_test_pass_client(raw_client, &api).await;

        let handled = api.handler_with_method(Method::GET, "/core/v4/events/latest", move |_| {
            success(serde_json::json!({ "Code": 1000, "EventID": "event-initial" }))
        });

        bootstrap_event_sync(&client).await;

        assert_hit!(handled);
        assert_eq!(
            Some("event-initial".to_string()),
            read_cursor(&client).await.unwrap()
        );
    }

    #[muon_test::test]
    async fn subsequent_run_no_key_changes(server: muon_test::Server) {
        let (raw_client, api) = server.client::<()>();
        let client = make_test_pass_client(raw_client, &api).await;

        write_cursor(&client, "event-abc").await.unwrap();

        let handled =
            api.handler_with_method(Method::GET, "/core/v4/events/event-abc", move |_| {
                success(serde_json::json!({ "Code": 1000, "EventID": "event-xyz", "More": 0 }))
            });

        bootstrap_event_sync(&client).await;

        assert_hit!(handled);
        assert_eq!(
            Some("event-xyz".to_string()),
            read_cursor(&client).await.unwrap()
        );
    }

    #[muon_test::test]
    async fn subsequent_run_keys_changed(server: muon_test::Server) {
        let (raw_client, api) = server.client::<()>();
        let client = make_test_pass_client(raw_client, &api).await;

        write_cursor(&client, "event-abc").await.unwrap();

        let handled =
            api.handler_with_method(Method::GET, "/core/v4/events/event-abc", move |_| {
                success(serde_json::json!({
                    "Code": 1000, "EventID": "event-xyz", "More": 0,
                    "User": { "Keys": [{}] }
                }))
            });

        bootstrap_event_sync(&client).await;

        assert_hit!(handled);
        // Cursor is advanced immediately after clearing the key cache
        assert_eq!(
            Some("event-xyz".to_string()),
            read_cursor(&client).await.unwrap()
        );
    }
}
