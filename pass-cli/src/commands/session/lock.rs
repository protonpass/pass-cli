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

use crate::helpers::CliPassClient as PassClient;
use anyhow::{Context, Result, bail};
use parking_lot::RwLock;
use pass::SessionLockState;
use pass::auth::store::PassSessionStore;
use std::sync::Arc;

pub async fn run(client: PassClient, store: Arc<RwLock<PassSessionStore>>) -> Result<()> {
    if client.is_agent_session() {
        bail!("Session lock is not available for agent sessions");
    }
    match super::sync_session_lock_state(&client, &store).await? {
        SessionLockState::NoLock => {
            bail!("Session has no lock. Create one first with `pass-cli session create-lock`")
        }
        SessionLockState::Locked => {
            println!("Session is already locked");
            return Ok(());
        }
        SessionLockState::Unlocked { .. } => {}
    }

    client
        .force_lock_session()
        .await
        .context("Error locking session")?;

    println!("Session locked successfully");
    Ok(())
}
