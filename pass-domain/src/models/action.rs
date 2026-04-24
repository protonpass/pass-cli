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

#[derive(Clone, Copy, Debug, serde::Serialize, Default)]
pub enum EventAction {
    ItemRead,
    #[default]
    Unknown,
}

impl EventAction {
    const ITEM_READ: u64 = 31;
    const UNKNOWN: u64 = 9999;

    pub fn value(&self) -> u64 {
        match self {
            Self::ItemRead => Self::ITEM_READ,
            Self::Unknown => Self::UNKNOWN,
        }
    }

    pub fn from(value: u64) -> Option<Self> {
        match value {
            Self::ITEM_READ => Some(Self::ItemRead),
            _ => None,
        }
    }
}
