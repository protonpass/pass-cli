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
