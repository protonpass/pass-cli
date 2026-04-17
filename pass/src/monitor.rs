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
use pass_domain::crypto::EncryptionTag;
use pass_domain::{AccountType, ActionPayload, ActionPayloadContent, PersonalAccessTokenId};

pub const MAX_REASON_LENGTH: usize = 300;
const PAGE_SIZE: usize = 100;

#[derive(Clone, Copy, Debug, serde::Serialize, Default)]
pub enum PatMonitorAction {
    ItemRead,
    #[default]
    Unknown,
}

impl PatMonitorAction {
    const ITEM_READ: u64 = 31;
    const UNKNOWN: u64 = 9999;

    pub fn value(&self) -> u64 {
        match self {
            PatMonitorAction::ItemRead => Self::ITEM_READ,
            PatMonitorAction::Unknown => Self::UNKNOWN,
        }
    }

    pub fn from(value: u64) -> Option<Self> {
        match value {
            Self::ITEM_READ => Some(Self::ItemRead),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct PatMonitorRequest {
    #[serde(rename = "Records")]
    pub records: Vec<PatMonitorRecord>,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct PatMonitorRecord {
    #[serde(rename = "VaultID")]
    pub vault_id: String,
    #[serde(rename = "ObjectID")]
    pub object_id: Option<String>,
    #[serde(rename = "Action")]
    pub action: u64,
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
    #[serde(rename = "VaultID")]
    vault_id: String,
    #[serde(rename = "ObjectID")]
    object_id: Option<String>,
    #[serde(rename = "Action")]
    action: u64,
    #[serde(rename = "Payload")]
    payload: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PatMonitorEntry {
    pub record_id: String,
    pub vault_id: String,
    pub object_id: Option<String>,
    pub action: PatMonitorAction,
    pub payload: DecryptedMonitorPayload,
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
                let payload = decrypt_monitor_payload(&rec.payload, &pat_key)
                    .context("Error decrypting monitor payload")?;
                all_records.push(PatMonitorEntry {
                    record_id: rec.pat_monitor_record_id,
                    vault_id: rec.vault_id,
                    object_id: rec.object_id,
                    action: PatMonitorAction::from(rec.action).unwrap_or_default(),
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

        if reason.len() > MAX_REASON_LENGTH {
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

        let encrypted =
            pass_domain::crypto::encrypt(&serialized, &pat_key, EncryptionTag::ActionPayload)
                .map_err(|e| {
                    error!("Error encrypting action payload: {e:#}");
                    anyhow!("Error encrypting action payload")
                })?;

        let encoded = b64_encode(encrypted);

        let request = PatMonitorRequest {
            records: vec![PatMonitorRecord {
                vault_id: share.vault_id.to_string(),
                object_id: Some(item_details.item.id.to_string()),
                action: PatMonitorAction::ItemRead.value(),
                payload: encoded,
            }],
        };

        self.send_pat_monitor_request(request)
            .await
            .context("Error sending agent monitor request")?;
        Ok(())
    }

    async fn send_pat_monitor_request(&self, request: PatMonitorRequest) -> Result<()> {
        let req = POST!("/pass/v1/pat/monitor")
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

    let decrypted = pass_domain::crypto::decrypt(&encrypted, pat_key, EncryptionTag::ActionPayload)
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
