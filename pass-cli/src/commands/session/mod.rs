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
use anyhow::{Context, Result};
use clap::Subcommand;
use parking_lot::RwLock;
use pass::SessionLockState;
use pass::auth::store::PassSessionStore;
use std::sync::Arc;

pub mod create_lock;
pub mod lock;
pub mod remove_lock;
pub mod unlock;

#[derive(Subcommand)]
pub enum SessionCommands {
    #[command(about = "Create a lock for the current session with a lock code")]
    CreateLock {
        #[arg(
            long,
            help = "Seconds of inactivity after which the session locks (min 30, max 900)",
            default_value = "300"
        )]
        idle_timeout: u32,
    },
    #[command(about = "Lock the current session now (requires an existing lock)")]
    Lock,
    #[command(about = "Unlock the current session with a lock code")]
    Unlock,
    #[command(about = "Remove the session lock entirely")]
    RemoveLock,
}

pub async fn run(
    subcommand: SessionCommands,
    client: PassClient,
    store: Arc<RwLock<PassSessionStore>>,
) -> Result<()> {
    match subcommand {
        SessionCommands::CreateLock {
            idle_timeout: lock_time,
        } => create_lock::run(client, store, Some(lock_time)).await,
        SessionCommands::Lock => lock::run(client, store).await,
        SessionCommands::Unlock => unlock::run(client, store).await,
        SessionCommands::RemoveLock => remove_lock::run(client, store).await,
    }
}

fn refreshed_lock_after_seconds(state: SessionLockState, cached: Option<u32>) -> Option<u32> {
    match state {
        SessionLockState::NoLock => None,
        SessionLockState::Unlocked {
            unlocked_secs: Some(secs),
        } => Some(secs),
        _ => cached,
    }
}

pub async fn sync_session_lock_state(
    client: &PassClient,
    store: &Arc<RwLock<PassSessionStore>>,
) -> Result<SessionLockState> {
    if client.is_agent_session() || client.is_pat_account() {
        return Ok(SessionLockState::NoLock);
    }

    let state = client
        .get_session_lock_state()
        .await
        .context("Error checking session lock")?;

    let cached = store.read().get_session_lock_after_seconds();
    let refreshed = refreshed_lock_after_seconds(state, cached);
    if refreshed != cached {
        let snapshot = {
            let mut guard = store.write();
            guard.set_session_lock_after_seconds(refreshed);
            guard.clone()
        };
        snapshot
            .persist_now()
            .await
            .context("Error persisting session")?;
    }

    Ok(state)
}

pub struct SessionLockStatus {
    pub has_lock: bool,
    pub lock_after_seconds: Option<u32>,
}

pub async fn session_lock_status(
    client: &PassClient,
    store: &Arc<RwLock<PassSessionStore>>,
) -> SessionLockStatus {
    let has_lock = match sync_session_lock_state(client, store).await {
        Ok(state) => state.has_lock(),
        Err(e) => {
            tracing::debug!("Could not check session lock with the server: {e:#}");
            store.read().get_session_lock_after_seconds().is_some()
        }
    };
    SessionLockStatus {
        has_lock,
        lock_after_seconds: store.read().get_session_lock_after_seconds(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_lock_clears_cached_value() {
        assert_eq!(
            refreshed_lock_after_seconds(SessionLockState::NoLock, Some(600)),
            None
        );
    }

    #[test]
    fn unlocked_lock_uses_server_value() {
        let state = SessionLockState::Unlocked {
            unlocked_secs: Some(300),
        };
        assert_eq!(refreshed_lock_after_seconds(state, None), Some(300));
        assert_eq!(refreshed_lock_after_seconds(state, Some(600)), Some(300));
    }

    #[test]
    fn locked_session_keeps_cached_value() {
        assert_eq!(
            refreshed_lock_after_seconds(SessionLockState::Locked, Some(600)),
            Some(600)
        );
        assert_eq!(
            refreshed_lock_after_seconds(SessionLockState::Locked, None),
            None
        );
    }

    #[test]
    fn unlocked_lock_without_value_keeps_cached_value() {
        let state = SessionLockState::Unlocked {
            unlocked_secs: None,
        };
        assert_eq!(refreshed_lock_after_seconds(state, Some(600)), Some(600));
    }
}
