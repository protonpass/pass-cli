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
use anyhow::{bail, Context, Result};
use pass_auth::store::PassSessionStore;
use std::sync::{Arc, RwLock};

pub async fn run(client: PassClient, store: Arc<RwLock<PassSessionStore>>) -> Result<()> {
    let is_locked = store
        .read()
        .expect("store rwlock poisoned")
        .has_session_lock();
    if !is_locked {
        bail!("Session is not locked");
    }

    let pin = ask_for_input("Enter PIN: ", true).context("Error reading PIN")?;

    client
        .remove_session_lock(&pin)
        .await
        .context("Error removing session lock")?;

    // Update the local session state to mark it as unlocked
    {
        let mut store_guard = store.write().expect("store rwlock poisoned");
        store_guard.set_has_session_lock(false);
        (*store_guard).persist_now().await?;
    }

    println!("Session lock removed successfully");
    Ok(())
}
