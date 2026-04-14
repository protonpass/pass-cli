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

use crate::auth::auth_helpers::create_authenticator;
use crate::features::CliClientFeatures;
use crate::helpers::CliPassClient as PassClient;
use crate::helpers::{PassClientExt, SessionExt};
use anyhow::{Context, Result};
use pass::FirstTimeSetupKey;
use pass_auth::PassSessionStore;
use pass_auth::os::ProdClient;
use std::sync::Arc;
use std::sync::RwLock;

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
    client: ProdClient,
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
