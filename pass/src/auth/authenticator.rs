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

use crate::auth::callbacks::{AuthEventHandler, CredentialProvider};
use crate::auth::config::ClientConfig;
use crate::auth::runtime::{AuthRuntime, ClientWithStore};
use crate::auth::storage::SessionStorage;
use crate::auth::store::PassSessionStore;
use crate::auth::{extra_password, personal_access_token, post_login, web_login};
use crate::domain::{AccountType, LocalKeyProvider};
use crate::{ClientFeatures, FirstTimeSetupKey, PassClient};
use anyhow::{Context as _, Result, bail};
use muon::common::sdk::Sdk;
use parking_lot::RwLock;
use std::sync::Arc;
use zeroize::Zeroizing;

pub struct Authenticator<R: AuthRuntime> {
    key_provider: Arc<dyn LocalKeyProvider>,
    storage: Arc<dyn SessionStorage>,
    pub(crate) event_handler: Arc<dyn AuthEventHandler>,
    credential_provider: Arc<dyn CredentialProvider>,
    config: ClientConfig,
    pub(crate) sdk: Sdk,
    runtime: R,
}

pub(crate) async fn persist_store(store: &Arc<RwLock<PassSessionStore>>) -> Result<()> {
    let store_snapshot = {
        let store_guard = store.read();
        store_guard.clone()
    };

    store_snapshot
        .persist_now()
        .await
        .context("Error persisting session store")?;

    Ok(())
}

impl<R: AuthRuntime> Authenticator<R> {
    pub fn new(
        key_provider: Arc<dyn LocalKeyProvider>,
        storage: Arc<dyn SessionStorage>,
        event_handler: Arc<dyn AuthEventHandler>,
        credential_provider: Arc<dyn CredentialProvider>,
        config: ClientConfig,
        sdk: Sdk,
        runtime: R,
    ) -> Self {
        Self {
            key_provider,
            storage,
            event_handler,
            credential_provider,
            config,
            sdk,
            runtime,
        }
    }

    pub async fn create_client(&self) -> Result<ClientWithStore<R::Context>> {
        self.runtime
            .create_client(
                self.key_provider.clone(),
                self.storage.clone(),
                &self.config,
                &self.sdk,
            )
            .await
    }

    pub async fn login_web(
        &self,
        client: muon::Client<R::Context>,
        client_features: Arc<dyn ClientFeatures>,
        store: Arc<RwLock<PassSessionStore>>,
    ) -> Result<(PassClient<R::Context>, Vec<u8>)> {
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
            let mut store_guard = store.write();
            store_guard.set_account_type(AccountType::User);
        }

        let session = client
            .new_session_without_credentials(())
            .await
            .context("Error creating session")?;

        let login_result = web_login::perform_web_login(
            &session,
            store.clone(),
            self.event_handler.clone(),
            &self.sdk,
            self.config.child_client_id(),
            self.runtime.time(),
        )
        .await
        .context("Error in web login flow")?;

        session.remove_auth().await;
        let _ = client
            .new_session_with_credentials((), login_result.credentials)
            .await
            .context("Error storing web login session")?;

        persist_store(&store).await?;

        let pass_client =
            PassClient::new(client, client_features, AccountType::User, self.sdk.clone());

        let needs_extra_password = {
            let store_guard = store.read();
            store_guard.needs_extra_password()
        };

        if needs_extra_password {
            info!("Account needs Pass extra password");
            let session = pass_client.get_session().await?;
            extra_password::handle_extra_password(
                &session,
                self.credential_provider.clone(),
                self.event_handler.clone(),
                &self.sdk,
            )
            .await?;
        }

        let user_info = pass_client.get_info().await.context("Error getting info")?;

        self.event_handler
            .on_info(&format!("Login performed by {}", user_info.user.email))
            .await?;

        let passphrase = login_result.session_payload.passphrase();
        Ok((pass_client, passphrase))
    }

    pub async fn login_personal_access_token(
        &self,
        client: muon::Client<R::Context>,
        client_features: Arc<dyn ClientFeatures>,
        store: Arc<RwLock<PassSessionStore>>,
        token: Option<String>,
    ) -> Result<(PassClient<R::Context>, Zeroizing<Vec<u8>>)> {
        let session = client.get_session(()).await;
        if let Some(session) = session
            && session.is_authenticated().await
        {
            self.event_handler
                .on_warning("Client is already authenticated. Log out if you want to log in again")
                .await?;
            bail!("Already authenticated");
        }

        let token = match token {
            Some(t) => t,
            None => self.credential_provider.get_personal_access_token().await?,
        };

        {
            let mut store_guard = store.write();
            store_guard.set_account_type(AccountType::PersonalAccessToken);
        }

        let session = client
            .new_session_without_credentials(())
            .await
            .context("Error creating session")?;

        let login_result =
            personal_access_token::perform_personal_access_token_login(&session, &token, &self.sdk)
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

        let mut pass_client = PassClient::new(
            client,
            client_features,
            AccountType::PersonalAccessToken,
            self.sdk.clone(),
        );

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

        {
            let mut store_guard = store.write();
            store_guard.set_account_type(account_type);
        }

        if account_type == AccountType::AgentSession {
            pass_client.set_account_type(AccountType::AgentSession);
        }

        persist_store(&store).await?;

        Ok((pass_client, login_result.personal_access_token_key))
    }

    pub async fn complete_login(
        &self,
        pass_client: &PassClient<R::Context>,
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
