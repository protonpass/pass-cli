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

use crate::{EventId, UserEvents};
use anyhow::{Error, Result};

pub enum ContinuationStrategy {
    Continue,
    Break { err: Error },
}

#[async_trait::async_trait]
pub trait UserEventsHandler: Send + Sync {
    async fn get_last_user_event_id(&self) -> Result<Option<EventId>>;
    async fn set_last_user_event_id(&self, event_id: EventId) -> Result<()>;
    async fn tick(&self);
    async fn on_event(&self, event: UserEvents) -> Result<()>;
    async fn on_error(&self, err: Error) -> Result<ContinuationStrategy>;
    async fn on_event_fetch_success(&self) {}
}
