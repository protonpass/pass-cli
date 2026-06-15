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
use crate::utils::ask_for_input;
use anyhow::{bail, ensure, Context, Result};
use pass_auth::store::PassSessionStore;
use std::sync::{Arc, RwLock};

const MIN_LOCK_TIME: u32 = 30;
const MAX_LOCK_TIME: u32 = 900;
const DEFAULT_LOCK_TIME: u32 = 300;

pub fn validate_lock_time(lock_time: u32) -> Result<u32> {
    ensure!(
        lock_time >= MIN_LOCK_TIME,
        "Lock time must be at least {MIN_LOCK_TIME} seconds"
    );
    ensure!(
        lock_time <= MAX_LOCK_TIME,
        "Lock time must be at most {MAX_LOCK_TIME} seconds"
    );
    Ok(lock_time)
}

pub async fn run(
    client: PassClient,
    store: Arc<RwLock<PassSessionStore>>,
    lock_time: Option<u32>,
) -> Result<()> {
    let is_locked = store
        .read()
        .expect("store rwlock poisoned")
        .has_session_lock();
    if is_locked {
        bail!("Session already has a lock");
    }
    let pin = ask_for_input("Enter PIN: ", true).context("Error reading PIN")?;

    let lock_time = lock_time.unwrap_or(DEFAULT_LOCK_TIME);
    let lock_time = validate_lock_time(lock_time)?;

    client
        .lock_session(&pin, lock_time)
        .await
        .context("Error locking session")?;

    // Update the local session state to mark it as having a lock
    {
        let mut store_guard = store.write().expect("store rwlock poisoned");
        store_guard.set_has_session_lock(true);
        (*store_guard).persist_now().await?;
    }

    println!("Session locked successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_lock_time_minimum() {
        assert_eq!(validate_lock_time(MIN_LOCK_TIME).unwrap(), MIN_LOCK_TIME);
    }

    #[test]
    fn test_validate_lock_time_maximum() {
        assert_eq!(validate_lock_time(MAX_LOCK_TIME).unwrap(), MAX_LOCK_TIME);
    }

    #[test]
    fn test_validate_lock_time_default() {
        assert_eq!(
            validate_lock_time(DEFAULT_LOCK_TIME).unwrap(),
            DEFAULT_LOCK_TIME
        );
    }

    #[test]
    fn test_validate_lock_time_below_minimum() {
        assert!(validate_lock_time(MIN_LOCK_TIME - 1).is_err());
    }

    #[test]
    fn test_validate_lock_time_above_maximum() {
        assert!(validate_lock_time(MAX_LOCK_TIME + 1).is_err());
    }
}
