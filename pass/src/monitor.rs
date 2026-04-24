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
use crate::item::get_one::ItemDetails;
use crate::utils::{b64_decode, b64_encode};
use crate::{PassClient, PassClientContext};
use anyhow::{Context, Result, anyhow};
use muon::{GET, POST};
use pass_domain::crypto::{self, EncryptionTag};
use pass_domain::{
    AccountType, ActionPayload, ActionPayloadContent, EventAction, PersonalAccessTokenId,
};

pub const MAX_REASON_LENGTH: usize = 300;
const PAGE_SIZE: usize = 100;

#[derive(Clone, Debug, serde::Serialize)]
pub struct PatMonitorRequest {
    #[serde(rename = "Records")]
    pub records: Vec<PatMonitorRecord>,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct PatMonitorRecord {
    #[serde(rename = "VaultID")]
    pub vault_id: String,
    #[serde(rename = "ItemID")]
    pub item_id: Option<String>,
    #[serde(rename = "Payload")]
    pub payload: String,
}

#[derive(Debug, serde::Deserialize)]
struct PatMonitorListApiResponse {
    #[serde(rename = "Actions")]
    actions: PatMonitorListApiActions,
}

#[derive(Debug, serde::Deserialize)]
struct PatMonitorListApiActions {
    #[serde(rename = "Records")]
    records: Vec<PatMonitorApiRecord>,
    #[serde(rename = "NextSince")]
    next_since: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct PatMonitorApiRecord {
    #[serde(rename = "PatMonitorRecordID")]
    pat_monitor_record_id: String,
    #[serde(rename = "Action")]
    action: u64,
    #[serde(rename = "VaultID")]
    vault_id: String,
    #[serde(rename = "ObjectID")]
    object_id: String,
    #[serde(rename = "Payload")]
    payload: String,
    #[serde(rename = "ActionTime")]
    action_time: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PatMonitorEntry {
    pub record_id: String,
    pub vault_id: String,
    pub object_id: String,
    pub action: EventAction,
    pub payload: DecryptedMonitorPayload,
    pub action_time: jiff::Timestamp,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DecryptedMonitorPayload {
    pub reason: String,
    pub vault_name: Option<String>,
    pub item_name: Option<String>,
}

impl<C: PassClientContext> PassClient<C> {
    pub async fn list_pat_monitor(
        &self,
        pat_id: &PersonalAccessTokenId,
        max_results: usize,
    ) -> Result<Vec<PatMonitorEntry>> {
        let pat_key = match self.account_type {
            AccountType::User => self
                .get_personal_access_token_key(pat_id)
                .await
                .context("Error getting personal access token key")?,
            AccountType::AgentSession | AccountType::PersonalAccessToken => self
                .get_local_personal_access_token_key()
                .await
                .context("Error getting local PAT key")?,
        };

        let mut all_records: Vec<PatMonitorEntry> = Vec::new();
        let mut since: Option<String> = None;

        loop {
            let remaining = max_results.saturating_sub(all_records.len());
            if remaining == 0 {
                break;
            }

            let page_size = remaining.min(PAGE_SIZE);
            let mut req =
                GET!("/pass/v1/pat/monitor/{pat_id}").query(("PageSize", page_size.to_string()));

            if let Some(ref cursor) = since {
                req = req.query(("Since", cursor.clone()));
            }

            let res = self
                .send(req)
                .await
                .context("Error fetching PAT monitor records")?;

            let response: PatMonitorListApiResponse = assert_response!(res);
            let actions = response.actions;
            let next_since = actions.next_since;
            let fetched = actions.records.len();

            for rec in actions.records {
                let action = EventAction::from(rec.action).unwrap_or_default();
                let payload = match decrypt_monitor_payload(&rec.payload, &pat_key) {
                    Ok(payload) => payload,
                    Err(e) => {
                        error!("Unable to decrypt payload: {e:#}");
                        DecryptedMonitorPayload {
                            reason: "Unable to decrypt".to_string(),
                            vault_name: None,
                            item_name: None,
                        }
                    }
                };

                let action_time = match jiff::Timestamp::from_second(rec.action_time) {
                    Ok(time) => time,
                    Err(e) => {
                        warn!("Could not parse timestamp {}: {:#}", rec.action_time, e);
                        jiff::Timestamp::constant(0, 0)
                    }
                };

                all_records.push(PatMonitorEntry {
                    record_id: rec.pat_monitor_record_id,
                    vault_id: rec.vault_id,
                    object_id: rec.object_id,
                    action_time,
                    action,
                    payload,
                });
            }

            if fetched == 0 || next_since.is_none() {
                break;
            }
            since = next_since;
        }

        // API returns records newest-first, reverse to oldest-first for display
        all_records.reverse();
        Ok(all_records)
    }

    pub async fn send_item_accessed_event(
        &self,
        item_details: &ItemDetails,
        reason: &str,
    ) -> Result<()> {
        if !self.is_agent_session() {
            return Err(anyhow!(
                "`send_item_accessed_event` can only be called from an agent session"
            ));
        }

        if reason.chars().count() > MAX_REASON_LENGTH {
            return Err(anyhow!(
                "reason is too long, please keep it under {MAX_REASON_LENGTH} characters"
            ));
        }

        let share = self.get_share(&item_details.item.share_id).await?;
        let vault_name = if share.is_vault_share() {
            let vault_content = self
                .open_vault_share_content_from_vault_share(&share)
                .await
                .context("Error opening share content")?;
            Some(vault_content.name.to_string())
        } else {
            None
        };

        let payload = ActionPayload {
            content: ActionPayloadContent::AgentAccessItem {
                reason: reason.to_string(),
                vault_name,
                item_name: Some(item_details.item.content.title.to_string()),
            },
        };

        let serialized = payload.serialize()?;
        let pat_key = self
            .get_local_personal_access_token_key()
            .await
            .context("Error getting local key")?;

        let encrypted = crypto::encrypt(&serialized, &pat_key, EncryptionTag::ActionPayload)
            .map_err(|e| {
                error!("Error encrypting action payload: {e:#}");
                anyhow!("Error encrypting action payload")
            })?;

        let encoded = b64_encode(encrypted);

        let request = PatMonitorRequest {
            records: vec![PatMonitorRecord {
                vault_id: share.vault_id.to_string(),
                item_id: Some(item_details.item.id.to_string()),
                payload: encoded,
            }],
        };

        self.send_pat_monitor_request(request)
            .await
            .context("Error sending agent monitor request")?;
        Ok(())
    }

    async fn send_pat_monitor_request(&self, request: PatMonitorRequest) -> Result<()> {
        let req = POST!("/pass/v1/pat/monitor/read")
            .body_json(request)
            .context("Error creating request to send action payload")?;

        let res = self
            .send(req)
            .await
            .context("Error sending action payload request")?;
        let response: CodeResponse = assert_response!(res);
        response.success_guard()?;

        Ok(())
    }
}

fn decrypt_monitor_payload(encoded: &str, pat_key: &[u8]) -> Result<DecryptedMonitorPayload> {
    let encrypted = b64_decode(encoded).context("Error base64-decoding monitor payload")?;

    let decrypted = crypto::decrypt(&encrypted, pat_key, EncryptionTag::ActionPayload)
        .map_err(|e| anyhow!("Error decrypting monitor payload: {e:?}"))?;

    let action_payload =
        ActionPayload::deserialize(&decrypted).context("Error deserializing monitor payload")?;

    match action_payload.content {
        ActionPayloadContent::AgentAccessItem {
            reason,
            vault_name,
            item_name,
        } => Ok(DecryptedMonitorPayload {
            reason,
            vault_name,
            item_name,
        }),
    }
}
