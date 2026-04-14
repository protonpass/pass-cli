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

use crate::config::PostLoginConfig;
use crate::os::ProdContext;
use crate::store::PassSessionStore;
use anyhow::{Context, Result};
use pass::{CreateVaultArgs, FirstTimeSetupKey, PassClient};
use std::sync::{Arc, RwLock};

pub async fn perform_post_login_setup(
    client: &PassClient<ProdContext>,
    key: FirstTimeSetupKey,
    config: &PostLoginConfig,
) -> Result<()> {
    client
        .perform_first_time_setup_with_key(key)
        .await
        .context("Error performing first time setup")?;

    info!("Successfully finished setup for user");

    if config.create_default_vault {
        let vaults = client.list_vaults().await.context("Couldn't list vaults")?;
        if vaults.is_empty() {
            info!("Could not find any vault. Creating a default one");
            let args = CreateVaultArgs::new(config.default_vault_name.clone())
                .context("Error creating default vault args")?;
            let (share_id, _) = client
                .create_vault(args)
                .await
                .context("Error creating default vault")?;
            info!("Created vault with id: {}", share_id);
        }
    }

    Ok(())
}

pub async fn get_user_id_from_store(store: Arc<RwLock<PassSessionStore>>) -> Result<String> {
    let store_guard = store.read().expect("store rwlock poisoned");
    let auth_guard = store_guard.auth.lock().expect("auth mutex poisoned");
    if let Some(auth) = auth_guard.as_ref() {
        match auth {
            muon::auth::Auth::Internal { user_id, .. } => Ok(user_id.clone()),
            muon::auth::Auth::External { user_id, .. } => Ok(user_id.clone()),
            muon::auth::Auth::None => Err(anyhow::anyhow!("No user ID: auth is None")),
            muon::auth::Auth::Anonymous { .. } => {
                Err(anyhow::anyhow!("No user ID: anonymous auth"))
            }
        }
    } else {
        Err(anyhow::anyhow!("No auth in store"))
    }
}
