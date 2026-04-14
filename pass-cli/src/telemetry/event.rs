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

use pass_domain::TelemetryEvent;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct CommandEvent {
    pub command: String,
}

impl CommandEvent {
    pub fn new(command: &str) -> Self {
        Self {
            command: command.to_string(),
        }
    }
}

impl TelemetryEvent for CommandEvent {
    fn event_type(&self) -> String {
        "command.execute".to_string()
    }

    fn dimensions(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.insert("executedCommand".to_string(), self.command.clone());
        map
    }
}
