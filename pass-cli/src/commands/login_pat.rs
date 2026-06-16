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
use crate::helpers::{CliPassClient as PassClient, PassClientExt, SessionExt};
use anyhow::{Context, Result};
use parking_lot::RwLock;
use pass::FirstTimeSetupKey;
use pass_auth::Authenticator;
use pass_auth::PassSessionStore;
use pass_auth::os::ProdClient;
use std::sync::Arc;

pub async fn login_personal_access_token(
    authenticator: Authenticator,
    token_string_arg: Option<String>,
    client: ProdClient,
    client_features: Arc<CliClientFeatures>,
    store: Arc<RwLock<PassSessionStore>>,
) -> Result<PassClient> {
    // Perform personal access token login
    let (pass_client, personal_access_token_key) = authenticator
        .login_personal_access_token(
            client,
            client_features.clone(),
            store.clone(),
            token_string_arg,
        )
        .await?;

    // Perform first-time setup with the personal access token key
    let user_id = store.get_user_id().await?;
    let client_features = pass_client.get_cli_client_features()?;
    client_features.set_user_id(Some(user_id)).await;

    pass_client
        .perform_first_time_setup_with_key(FirstTimeSetupKey::PersonalAccessToken(
            personal_access_token_key,
        ))
        .await
        .context("Error performing first time setup")?;

    Ok(pass_client)
}

pub async fn run(
    token_string_arg: Option<String>,
    client: ProdClient,
    client_features: Arc<CliClientFeatures>,
    store: Arc<RwLock<PassSessionStore>>,
) -> Result<()> {
    let authenticator = create_authenticator(client_features.clone())?;
    let pass_client = login_personal_access_token(
        authenticator,
        token_string_arg,
        client,
        client_features,
        store,
    )
    .await?;

    // Get and display personal access token name
    let personal_access_token_name = pass_client
        .get_personal_access_token_name()
        .await
        .context("Error getting personal access token name")?;

    println!(
        "Successfully logged in as personal access token: {}",
        personal_access_token_name
    );
    Ok(())
}
