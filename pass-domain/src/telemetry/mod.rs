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

use anyhow::Result;
use std::collections::HashMap;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum ItemType {
    Note,
    Login,
    Alias,
    CreditCard,
    Identity,
    SshKey,
    Wifi,
    Custom,
}

impl ItemType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ItemType::Note => "note",
            ItemType::Login => "login",
            ItemType::Alias => "alias",
            ItemType::CreditCard => "credit_card",
            ItemType::Identity => "identity",
            ItemType::SshKey => "ssh_key",
            ItemType::Wifi => "wifi",
            ItemType::Custom => "custom",
        }
    }

    pub fn from_content(content: &crate::ItemContent) -> Self {
        match content {
            crate::ItemContent::Note(_) => ItemType::Note,
            crate::ItemContent::Login(_) => ItemType::Login,
            crate::ItemContent::Alias(_) => ItemType::Alias,
            crate::ItemContent::CreditCard(_) => ItemType::CreditCard,
            crate::ItemContent::Identity(_) => ItemType::Identity,
            crate::ItemContent::SshKey(_) => ItemType::SshKey,
            crate::ItemContent::Wifi(_) => ItemType::Wifi,
            crate::ItemContent::Custom(_) => ItemType::Custom,
        }
    }
}

pub trait TelemetryEvent {
    fn event_type(&self) -> String;

    fn dimensions(&self) -> HashMap<String, String> {
        HashMap::new()
    }
}

#[derive(Debug, Clone)]
pub struct TelemetryEventData {
    pub event_type: String,
    pub dimensions: HashMap<String, String>,
    pub user_id: Option<String>,
    pub timestamp: i64,
}

#[async_trait::async_trait(?Send)]
pub trait TelemetryHandler: Send + Sync {
    async fn emit_telemetry(&self, event: &dyn TelemetryEvent) -> Result<()>;
}

/// No-op telemetry handler for testing or when telemetry is disabled
pub struct NoopTelemetryHandler;

#[async_trait::async_trait(?Send)]
impl TelemetryHandler for NoopTelemetryHandler {
    async fn emit_telemetry(&self, _event: &dyn TelemetryEvent) -> Result<()> {
        Ok(())
    }
}
