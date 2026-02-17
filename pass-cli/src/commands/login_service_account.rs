use crate::auth::auth_helpers::create_authenticator;
use crate::features::CliClientFeatures;
use crate::helpers::{PassClientExt, SessionExt};
use anyhow::{Context, Result};
use pass::{Client, FirstTimeSetupKey};
use pass_auth::PassSessionStore;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn run(
    token_string_arg: Option<String>,
    client: Client,
    client_features: Arc<CliClientFeatures>,
    store: Arc<RwLock<PassSessionStore>>,
) -> Result<()> {
    let authenticator = create_authenticator(client_features.clone())?;

    // Perform service account login
    let (pass_client, service_account_key) = authenticator
        .login_service_account(
            client,
            client_features.clone(),
            store.clone(),
            token_string_arg,
        )
        .await?;

    // Perform first-time setup with the service account key
    let user_id = store.get_user_id().await?;
    let client_features = pass_client.get_cli_client_features()?;
    client_features.set_user_id(Some(user_id)).await;

    pass_client
        .perform_first_time_setup_with_key(FirstTimeSetupKey::ServiceAccount(service_account_key))
        .await
        .context("Error performing first time setup")?;

    // Get and display service account name
    let service_account_name = pass_client
        .get_service_account_name()
        .await
        .context("Error getting service account name")?;

    println!(
        "Successfully logged in as service account: {}",
        service_account_name
    );
    Ok(())
}
