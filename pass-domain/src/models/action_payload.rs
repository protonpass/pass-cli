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
use crate::protos::action_payload::action_payload::{
    ActionPayload as ProtoActionPayload, AgentAccessItem, action_payload::Content as ProtoContent,
};
use anyhow::{Context, Result};

#[derive(Clone, Debug)]
pub struct ActionPayload {
    pub content: ActionPayloadContent,
}

impl ActionPayload {
    pub fn serialize(self) -> Result<Vec<u8>> {
        let as_proto = ProtoActionPayload::from(self);
        as_proto
            .to_vec()
            .context("Error serializing vault to proto")
    }

    pub fn deserialize(data: &[u8]) -> Result<Self> {
        let as_proto = ProtoActionPayload::decode_from_slice(data)
            .context("Error decoding Vault from proto")?;

        let mapped = Self::try_from(as_proto).context("Error deserializing action payload")?;
        Ok(mapped)
    }
}

impl From<ActionPayload> for ProtoActionPayload {
    fn from(payload: ActionPayload) -> Self {
        Self {
            content: Some(ProtoContent::from(payload.content)),
            special_fields: Default::default(),
        }
    }
}

impl TryFrom<ProtoActionPayload> for ActionPayload {
    type Error = anyhow::Error;
    fn try_from(payload: ProtoActionPayload) -> Result<Self> {
        match payload.content {
            Some(c) => Ok(Self {
                content: ActionPayloadContent::from(c),
            }),
            None => Err(anyhow::anyhow!("Got None for ActionPayload content")),
        }
    }
}

#[derive(Clone, Debug)]
pub enum ActionPayloadContent {
    AgentAccessItem {
        reason: String,
        vault_name: Option<String>,
        item_name: Option<String>,
    },
}

impl From<ActionPayloadContent> for ProtoContent {
    fn from(payload: ActionPayloadContent) -> Self {
        match payload {
            ActionPayloadContent::AgentAccessItem {
                reason,
                vault_name,
                item_name,
            } => ProtoContent::AgentAccessItem(AgentAccessItem {
                reason,
                vault_name: vault_name.unwrap_or_else(|| "Unknown vault".to_string()),
                item_name: item_name.unwrap_or_else(|| "Unknown item".to_string()),
                special_fields: Default::default(),
            }),
        }
    }
}

impl From<ProtoContent> for ActionPayloadContent {
    fn from(payload: ProtoContent) -> Self {
        match payload {
            ProtoContent::AgentAccessItem(agent_access_item) => Self::AgentAccessItem {
                reason: agent_access_item.reason,
                vault_name: some_if_not_empty(agent_access_item.vault_name),
                item_name: some_if_not_empty(agent_access_item.item_name),
            },
        }
    }
}

fn some_if_not_empty(value: String) -> Option<String> {
    if value.is_empty() { None } else { Some(value) }
}
