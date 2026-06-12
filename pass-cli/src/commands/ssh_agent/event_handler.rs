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
use pass_domain::{ContinuationStrategy, EventId, UserEvents, UserEventsHandler};
use tokio::sync::mpsc::Sender;
use tokio::sync::{Mutex, RwLock};

const MAX_ERRORS: u8 = 3;

pub struct SshAgentEventHandler {
    last_event_id: RwLock<Option<EventId>>,
    refresh_interval: u64,
    tx: Sender<UserEvents>,
    num_errors: Mutex<u8>,
}

impl SshAgentEventHandler {
    pub fn new(tx: Sender<UserEvents>, refresh_interval: u64) -> Self {
        Self {
            last_event_id: RwLock::new(None),
            refresh_interval,
            tx,
            num_errors: Mutex::new(0),
        }
    }
}

#[async_trait::async_trait]
impl UserEventsHandler for SshAgentEventHandler {
    async fn get_last_user_event_id(&self) -> Result<Option<EventId>> {
        Ok(self.last_event_id.read().await.clone())
    }

    async fn set_last_user_event_id(&self, event_id: EventId) -> Result<()> {
        self.last_event_id.write().await.replace(event_id);
        Ok(())
    }

    async fn tick(&self) {
        tokio::time::sleep(std::time::Duration::from_secs(self.refresh_interval)).await;
    }

    async fn on_event(&self, event: UserEvents) -> Result<()> {
        if let Err(e) = self.tx.send(event).await {
            warn!("Error sending event: {:?}", e);
        }

        Ok(())
    }

    async fn on_error(&self, err: anyhow::Error) -> Result<ContinuationStrategy> {
        warn!("Error on listening for events: {err:?}");
        let error_count = {
            let mut errors = self.num_errors.lock().await;
            *errors += 1;
            *errors
        };

        if error_count == MAX_ERRORS {
            return Ok(ContinuationStrategy::Break { err });
        }

        self.tick().await;
        Ok(ContinuationStrategy::Continue)
    }

    async fn on_event_fetch_success(&self) {
        let mut errors = self.num_errors.lock().await;
        *errors = 0;
    }
}
