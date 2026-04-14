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

use pass_domain::{ItemType, TelemetryEvent};
use std::collections::HashMap;

pub mod batch;
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
