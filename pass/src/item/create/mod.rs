use pass_domain::{ItemType, TelemetryEvent};
use std::collections::HashMap;

pub(crate) mod common;
pub mod credit_card;
pub mod custom;
pub mod identity;
pub mod login;
pub mod note;
pub mod ssh_key;
pub mod wifi;

#[derive(Clone, Debug)]
pub(crate) struct ItemCreatedEvent {
    pub item_type: ItemType,
}

impl TelemetryEvent for ItemCreatedEvent {
    fn event_type(&self) -> String {
        "item.creation".to_string()
    }

    fn dimensions(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.insert("itemType".to_string(), self.item_type.as_str().to_string());
        map
    }
}
