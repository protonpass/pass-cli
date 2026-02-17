use crate::auth::auth_helpers::create_authenticator;
use crate::features::CliClientFeatures;
use crate::helpers::{PassClientExt, SessionExt};
use anyhow::{Context, Result};
use pass::{Client, FirstTimeSetupKey, PassClient};
use pass_auth::PassSessionStore;
use std::sync::Arc;
use tokio::sync::RwLock;

#[cfg(feature = "no-login-restriction")]
async fn is_login_allowed(_client: &PassClient) -> Result<bool> {
    Ok(true)
}

#[cfg(not(feature = "no-login-restriction"))]
async fn is_login_allowed(client: &PassClient) -> Result<bool> {
    let can_use = client.can_use_cli().await?;
    if !can_use {
        warn!("Your account is not allowed to use the CLI");
    }

    Ok(can_use)
}

pub(crate) async fn after_login(
    client: &PassClient,
    key: FirstTimeSetupKey,
    store: Arc<RwLock<PassSessionStore>>,
) -> Result<()> {
    let login_allowed = is_login_allowed(client)
        .await
        .context("Error checking login permissions")?;
    if !login_allowed {
        eprintln!("Your account is not yet allowed to use our CLI");
        client.logout().await?;
        crate::commands::logout::force_logout().await?;
        std::process::exit(1);
    }

    let user_id = store.get_user_id().await?;
    let client_features = client.get_cli_client_features()?;
    client_features.set_user_id(Some(user_id)).await;

    // Use pass-auth's post_login with CLI-specific post-processing
    let config = pass_auth::PostLoginConfig::default();
    pass_auth::post_login::perform_post_login_setup(client, key, &config)
        .await
        .context("Error in post-login setup")?;

    Ok(())
}

pub async fn run(
    username: Option<&str>,
    interactive: bool,
    client: Client,
    client_features: Arc<CliClientFeatures>,
    store: Arc<RwLock<PassSessionStore>>,
) -> Result<()> {
    let authenticator = create_authenticator(client_features.clone())?;

    let (pass_client, key) = if interactive {
        // Interactive login
        let (client, password) = authenticator
            .login_interactive(
                client,
                client_features.clone(),
                store.clone(),
                username.map(|s| s.to_string()),
            )
            .await?;
        (client, FirstTimeSetupKey::UserPassword(password))
    } else {
        // Web login
        let (client, passphrase) = authenticator
            .login_web(client, client_features.clone(), store.clone())
            .await?;
        (client, FirstTimeSetupKey::Passphrase(passphrase))
    };

    after_login(&pass_client, key, store).await?;

    let user_info = pass_client.get_info().await?;
    println!("Successfully logged in as {}", user_info.user.email);

    Ok(())
}
