use crate::config::PostLoginConfig;
use crate::store::PassSessionStore;
use anyhow::{Context, Result};
use pass::{CreateVaultArgs, FirstTimeSetupKey, PassClient};
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn perform_post_login_setup(
    client: &PassClient,
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
    let store_guard = store.read().await;
    if let Some(auth) = store_guard.auth.read().await.as_ref() {
        match auth {
            muon::client::Auth::Internal { user_id, .. } => Ok(user_id.clone()),
            _ => Err(anyhow::anyhow!("No user ID in auth")),
        }
    } else {
        Err(anyhow::anyhow!("No auth in store"))
    }
}
