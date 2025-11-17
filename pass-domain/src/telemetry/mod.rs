use anyhow::Result;

/// Represents different types of items in the vault
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

    /// Convert from ItemContent to ItemType
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

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum TelemetryEvent {
    ItemCreated { item_type: ItemType },
    ItemUpdated { item_type: ItemType },
    ItemDeleted { item_type: ItemType },
    ItemMoved { item_type: ItemType },
    VaultCreated,
    VaultUpdated,
    VaultDeleted,
    Command { command: String },
}

impl TelemetryEvent {
    pub fn command(command: &str) -> Self {
        Self::Command {
            command: command.to_string(),
        }
    }

    pub fn event_type(&self) -> &'static str {
        match self {
            TelemetryEvent::ItemCreated { .. } => "item_created",
            TelemetryEvent::ItemUpdated { .. } => "item_updated",
            TelemetryEvent::ItemDeleted { .. } => "item_deleted",
            TelemetryEvent::ItemMoved { .. } => "item_moved",
            TelemetryEvent::VaultCreated => "vault_created",
            TelemetryEvent::VaultUpdated => "vault_updated",
            TelemetryEvent::VaultDeleted => "vault_deleted",
            TelemetryEvent::Command { .. } => "command",
        }
    }

    pub fn item_type(&self) -> Option<&ItemType> {
        match self {
            TelemetryEvent::ItemCreated { item_type }
            | TelemetryEvent::ItemUpdated { item_type }
            | TelemetryEvent::ItemDeleted { item_type }
            | TelemetryEvent::ItemMoved { item_type } => Some(item_type),
            TelemetryEvent::VaultCreated
            | TelemetryEvent::VaultUpdated
            | TelemetryEvent::VaultDeleted
            | TelemetryEvent::Command { .. } => None,
        }
    }
}

#[async_trait::async_trait(?Send)]
pub trait TelemetryHandler: Send + Sync {
    async fn emit_telemetry(&self, event: TelemetryEvent) -> Result<()>;
}

/// No-op telemetry handler for testing or when telemetry is disabled
pub struct NoopTelemetryHandler;

#[async_trait::async_trait(?Send)]
impl TelemetryHandler for NoopTelemetryHandler {
    async fn emit_telemetry(&self, _event: TelemetryEvent) -> Result<()> {
        Ok(())
    }
}
