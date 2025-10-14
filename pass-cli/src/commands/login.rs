use crate::client::authenticate_client;
use crate::features::CliClientFeatures;
use crate::store::PassSessionStore;
use crate::utils::get_base_dir;
use anyhow::{Context, Result};
use muon::Client;
use pass::{CreateVaultArgs, PassClient, PassPlan};
use std::sync::Arc;
use tokio::sync::RwLock;

#[cfg(feature = "no-login-restriction")]
fn is_login_allowed(_: &PassPlan) -> bool {
    true
}

#[cfg(not(feature = "no-login-restriction"))]
fn is_login_allowed(plan: &PassPlan) -> bool {
    debug!("Checking is_login_allowed with plan {:?}", plan);
    if plan.type_ == pass::PlanType::Free {
        debug!("Free plans are not allowed");
        return false;
    }

    match plan.internal_name.as_str() {
        "visionary2022" | "bundlepro2024" => true,
        _ => false,
    }
}

pub async fn run(
    username: &str,
    client: Client,
    store: Arc<RwLock<PassSessionStore>>,
) -> Result<()> {
    if client.is_authenticated().await {
        info!("Client is already authenticated. Log out if you want to log in again");
        return Ok(());
    }
    info!("Logging in user: {}", username);

    let authenticated_client = authenticate_client(client, username, store).await?;

    info!("Logged in user: {}", username);
    let base_dir = get_base_dir().context("Couldn't get base directory")?;
    let key_provider =
        Arc::new(CliClientFeatures::new(base_dir).context("Error creating client features")?);
    let client = PassClient::new(authenticated_client.client, key_provider);

    let info = client
        .get_user_access()
        .await
        .context("Error retrieving user access info")?;
    if !is_login_allowed(&info.plan) {
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
