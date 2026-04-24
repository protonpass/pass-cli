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

use crate::callbacks::{AuthEventHandler, CredentialProvider};
use crate::client_builder;
use crate::config::ClientConfig;
use crate::os::{ProdClient, ProdContext};
use crate::storage::SessionStorage;
use crate::store::PassSessionStore;
use crate::{extra_password, interactive_login, personal_access_token, post_login, web_login};
use anyhow::{Context, Result, bail};
use pass::{FirstTimeSetupKey, PassClient};
use pass_domain::{AccountType, LocalKeyProvider};
use std::sync::{Arc, RwLock};
use zeroize::Zeroizing;

pub struct Authenticator {
    key_provider: Arc<dyn LocalKeyProvider>,
    storage: Arc<dyn SessionStorage>,
    event_handler: Arc<dyn AuthEventHandler>,
    credential_provider: Arc<dyn CredentialProvider>,
    config: ClientConfig,
}

impl Authenticator {
    async fn persist_store(store: &Arc<RwLock<PassSessionStore>>) -> Result<()> {
        let store_snapshot = {
            let store_guard = store.read().expect("store rwlock poisoned");
            store_guard.clone()
        };

        store_snapshot
            .persist_now()
            .await
            .context("Error persisting session store")?;

        Ok(())
    }

    pub fn new(
        key_provider: Arc<dyn LocalKeyProvider>,
        storage: Arc<dyn SessionStorage>,
        event_handler: Arc<dyn AuthEventHandler>,
        credential_provider: Arc<dyn CredentialProvider>,
        config: ClientConfig,
    ) -> Self {
        Self {
            key_provider,
            storage,
            event_handler,
            credential_provider,
            config,
        }
    }

    pub async fn create_client(&self) -> Result<(ProdClient, Arc<RwLock<PassSessionStore>>)> {
        client_builder::create_client(
            self.key_provider.clone(),
            self.storage.clone(),
            &self.config,
        )
        .await
    }

    pub async fn login_web(
        &self,
        client: ProdClient,
        client_features: Arc<dyn pass_domain::ClientFeatures>,
        store: Arc<RwLock<PassSessionStore>>,
    ) -> Result<(PassClient<ProdContext>, Vec<u8>)> {
        // Check if already authenticated
        let session = client.get_session(()).await;
        if let Some(session) = session
            && session.is_authenticated().await
        {
            self.event_handler
                .on_warning("Client is already authenticated. Log out if you want to log in again")
                .await?;
            bail!("Already authenticated");
        }

        {
            let mut store_guard = store.write().expect("store rwlock poisoned");
            store_guard.set_account_type(AccountType::User);
        }

        // Create unauthenticated session
        let session = client
            .new_session_without_credentials(())
            .await
            .context("Error creating session")?;

        // Perform web login
        let login_result =
            web_login::perform_web_login(&session, store.clone(), self.event_handler.clone())
                .await
                .context("Error in web login flow")?;

        session.remove_auth().await;
        let _ = client
            .new_session_with_credentials((), login_result.credentials)
            .await
            .context("Error storing web login session")?;

        Self::persist_store(&store).await?;

        let pass_client = PassClient::new(client, client_features, AccountType::User);

        // Check if extra password is needed
        let needs_extra_password = {
            let store_guard = store.read().expect("store rwlock poisoned");
            store_guard.needs_extra_password()
        };

        if needs_extra_password {
            info!("Account needs Pass extra password");
            let session = pass_client.get_session().await?;
            extra_password::handle_extra_password(
                &session,
                self.credential_provider.clone(),
                self.event_handler.clone(),
            )
            .await?;
        }

        // Get user info
        let user_info = pass_client.get_info().await.context("Error getting info")?;

        self.event_handler
            .on_info(&format!("Login performed by {}", user_info.user.email))
            .await?;

        let passphrase = login_result.session_payload.passphrase();
        Ok((pass_client, passphrase))
    }

    pub async fn login_interactive(
        &self,
        client: ProdClient,
        client_features: Arc<dyn pass_domain::ClientFeatures>,
        store: Arc<RwLock<PassSessionStore>>,
        username: Option<String>,
    ) -> Result<(PassClient<ProdContext>, Zeroizing<String>)> {
        // Check if already authenticated
        let session = client.get_session(()).await;
        if let Some(session) = session
            && session.is_authenticated().await
        {
            self.event_handler
                .on_info("Client is already authenticated. Log out if you want to log in again")
                .await?;
            bail!("Already authenticated");
        }

        // Get username
        let username = match username {
            Some(u) => u,
            None => self.credential_provider.get_username().await?,
        };

        info!("Logging in user: {}", username);

        // Set account type in store for regular user login
        {
            let mut store_guard = store.write().expect("store rwlock poisoned");
            store_guard.set_account_type(AccountType::User);
        }

        // Perform interactive login
        let auth_result = interactive_login::perform_interactive_login(
            client,
            &username,
            store.clone(),
            self.credential_provider.clone(),
            self.event_handler.clone(),
        )
        .await
        .context("Error in interactive login flow")?;

        Self::persist_store(&store).await?;

        info!("Logged in user: {}", username);

        let pass_client = PassClient::new(auth_result.client, client_features, AccountType::User);

        self.event_handler
            .on_info(&format!("Successfully logged in as {}", username))
            .await?;

        Ok((pass_client, auth_result.password))
    }

    pub async fn login_personal_access_token(
        &self,
        client: ProdClient,
        client_features: Arc<dyn pass_domain::ClientFeatures>,
        store: Arc<RwLock<PassSessionStore>>,
        token: Option<String>,
    ) -> Result<(PassClient<ProdContext>, Zeroizing<Vec<u8>>)> {
        // Check if already authenticated
        let session = client.get_session(()).await;
        if let Some(session) = session
            && session.is_authenticated().await
        {
            self.event_handler
                .on_warning("Client is already authenticated. Log out if you want to log in again")
                .await?;
            bail!("Already authenticated");
        }

        // Get token
        let token = match token {
            Some(t) => t,
            None => self.credential_provider.get_personal_access_token().await?,
        };

        {
            let mut store_guard = store.write().expect("store rwlock poisoned");
            store_guard.set_account_type(AccountType::PersonalAccessToken);
        }

        // Create unauthenticated session
        let session = client
            .new_session_without_credentials(())
            .await
            .context("Error creating session")?;

        // Perform personal access token login
        let login_result =
            personal_access_token::perform_personal_access_token_login(&session, &token)
                .await
                .context("Error in personal access token login flow")?;

        self.event_handler
            .on_auth_success("Personal access token session created successfully")
            .await?;

        session.remove_auth().await;
        let _ = client
            .new_session_with_credentials((), login_result.credentials)
            .await
            .context("Error storing personal access token session")?;

        // Create an authenticated client so we can fetch PAT flags from the self endpoint
        let mut pass_client =
            PassClient::new(client, client_features, AccountType::PersonalAccessToken);

        // Determine if this PAT was issued for an agent by checking the flags from the self endpoint
        let account_type = match pass_client.get_personal_access_token_pass_agent().await {
            Ok(true) => AccountType::AgentSession,
            Ok(false) => AccountType::PersonalAccessToken,
            Err(e) => {
                warn!(
                    "Failed to fetch PAT flags from self endpoint, defaulting to PersonalAccessToken: {e:#}"
                );
                AccountType::PersonalAccessToken
            }
        };

        // Update store with the resolved account type
        {
            let mut store_guard = store.write().expect("store rwlock poisoned");
            store_guard.set_account_type(account_type);
        }

        if account_type == AccountType::AgentSession {
            pass_client.set_account_type(AccountType::AgentSession);
        }

        Self::persist_store(&store).await?;

        Ok((pass_client, login_result.personal_access_token_key))
    }

    pub async fn complete_login(
        &self,
        pass_client: &PassClient<ProdContext>,
        key: FirstTimeSetupKey,
    ) -> Result<()> {
        post_login::perform_post_login_setup(pass_client, key, &self.config.post_login_config)
            .await
            .context("Error in post-login setup")?;

        self.event_handler
            .on_auth_success("Login completed successfully")
            .await?;

        Ok(())
    }
}
