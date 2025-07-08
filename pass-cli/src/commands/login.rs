use crate::client::authenticate_client;
use crate::features::CliClientFeatures;
use crate::utils::get_base_dir;
use anyhow::{Context, Result};
use muon::Client;
use pass::{CreateVaultArgs, PassClient};
use std::sync::Arc;

pub async fn run(username: &str, client: Client) -> Result<()> {
    if client.is_authenticated().await {
        info!("Client is already authenticated. Log out if you want to log in again");
        return Ok(());
    }
    info!("Logging in user: {}", username);

    let authenticated_client = authenticate_client(client, username).await?;

    info!("Logged in user: {}", username);
    let base_dir = get_base_dir().context("Couldn't get base directory")?;
    let key_provider = Arc::new(CliClientFeatures::new(base_dir));
    let client = PassClient::new(authenticated_client.client, key_provider);
    client
        .setup_user_keys(&authenticated_client.password)
        .await
        .context("Couldn't setup user keys")?;

    info!("Successfully finished UserKey setup for user: {}", username);

    let vaults = client.list_vaults().await.context("Couldn't list vaults")?;
    if vaults.is_empty() {
        info!("Could not find any vault. Creating a default one");
        let args = CreateVaultArgs::new("Personal".to_string())
            .context("Error creating default vault args")?;
        let (share_id, _) = client
            .create_vault(args)
            .await
            .context("Error creating default vault")?;
        info!("Created vault with id: {}", share_id);
    }

    Ok(())
}
