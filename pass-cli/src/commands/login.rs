use crate::client::authenticate_client;
use crate::features::CliClientFeatures;
use crate::store::PassSessionStore;
use anyhow::{Context, Result};
use muon::Client;
use pass::{CreateVaultArgs, PassClient};
use std::sync::Arc;
use tokio::sync::RwLock;

#[cfg(feature = "no-login-restriction")]
async fn is_login_allowed(_client: &PassClient) -> Result<bool> {
    Ok(true)
}

#[cfg(not(feature = "no-login-restriction"))]
async fn is_login_allowed(client: &PassClient) -> Result<bool> {
    let ff = client
        .has_feature_flag(pass_domain::FeatureFlag::PassCanUseCli)
        .await
        .context("Error checking PassCanUseCli feature flag")?;
    if ff {
        return Ok(true);
    }

    let info = client
        .get_user_access()
        .await
        .context("Error retrieving user access info")?;

    let plan = info.plan;
    debug!("Checking is_login_allowed with plan {:?}", plan);
    if !plan.cli_allowed {
        warn!("Your account is not allowed to use the CLI");
    }

    Ok(plan.cli_allowed)
}

pub async fn run(
    username: &str,
    client: Client,
    client_features: Arc<CliClientFeatures>,
    store: Arc<RwLock<PassSessionStore>>,
) -> Result<()> {
    if client.is_authenticated().await {
        info!("Client is already authenticated. Log out if you want to log in again");
        return Ok(());
    }
    info!("Logging in user: {}", username);

    let authenticated_client = authenticate_client(client, username, store).await?;

    info!("Logged in user: {}", username);

    let client = PassClient::new(authenticated_client.client, client_features);
    let login_allowed = is_login_allowed(&client)
        .await
        .context("Error checking login permissions")?;
    if !login_allowed {
        eprintln!("Your account is not yet allowed to use our CLI");
        client.logout().await?;
        crate::commands::logout::run(client).await?;
        std::process::exit(1);
    }

    client
        .perform_first_time_setup(&authenticated_client.password)
        .await
        .context("Error performing first time setup")?;

    info!("Successfully finished setup for user: {}", username);

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

    println!("Successfully logged in as {username}");

    Ok(())
}
