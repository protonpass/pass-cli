use crate::callbacks::{AuthEventHandler, CredentialProvider};
use crate::client_builder;
use crate::config::ClientConfig;
use crate::storage::SessionStorage;
use crate::store::PassSessionStore;
use crate::{extra_password, interactive_login, post_login, service_account, web_login};
use anyhow::{Context, Result, bail};
use pass::{Client, FirstTimeSetupKey, PassClient};
use pass_domain::{AccountType, LocalKeyProvider};
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct Authenticator {
    key_provider: Arc<dyn LocalKeyProvider>,
    storage: Arc<dyn SessionStorage>,
    event_handler: Arc<dyn AuthEventHandler>,
    credential_provider: Arc<dyn CredentialProvider>,
    config: ClientConfig,
}

impl Authenticator {
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

    pub async fn create_client(&self) -> Result<(Client, Arc<RwLock<PassSessionStore>>)> {
        client_builder::create_client(
            self.key_provider.clone(),
            self.storage.clone(),
            &self.config,
        )
        .await
    }

    pub async fn login_web(
        &self,
        client: Client,
        client_features: Arc<dyn pass_domain::ClientFeatures>,
        store: Arc<RwLock<PassSessionStore>>,
    ) -> Result<(PassClient, Vec<u8>)> {
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

        // Create unauthenticated session
        let session = client
            .new_session_without_credentials(())
            .await
            .context("Error creating session")?;

        // Perform web login
        let session_payload =
            web_login::perform_web_login(session, store.clone(), self.event_handler.clone())
                .await
                .context("Error in web login flow")?;

        // Create a new PassClient (HACK to ensure store is fresh)
        let (client, store) = self.create_client().await?;
        let pass_client = PassClient::new(client, client_features, AccountType::User);

        // Check if extra password is needed
        let needs_extra_password = {
            let store_guard = store.read().await;
            store_guard.needs_extra_password().await
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

        let passphrase = session_payload.passphrase();
        Ok((pass_client, passphrase))
    }

    pub async fn login_interactive(
        &self,
        client: Client,
        client_features: Arc<dyn pass_domain::ClientFeatures>,
        store: Arc<RwLock<PassSessionStore>>,
        username: Option<String>,
    ) -> Result<(PassClient, String)> {
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

        // Set account type in store for regular user login
        {
            let mut store_guard = store.write().await;
            store_guard.set_account_type(AccountType::User);
        }

        info!("Logged in user: {}", username);

        let pass_client = PassClient::new(auth_result.client, client_features, AccountType::User);

        self.event_handler
            .on_info(&format!("Successfully logged in as {}", username))
            .await?;

        Ok((pass_client, auth_result.password))
    }

    pub async fn login_service_account(
        &self,
        client: Client,
        client_features: Arc<dyn pass_domain::ClientFeatures>,
        store: Arc<RwLock<PassSessionStore>>,
        token: Option<String>,
    ) -> Result<(PassClient, Vec<u8>)> {
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
            None => self.credential_provider.get_service_account_token().await?,
        };

        // Create unauthenticated session
        let session = client
            .new_session_without_credentials(())
            .await
            .context("Error creating session")?;

        // Perform service account login
        let service_account_key =
            service_account::perform_service_account_login(session, store.clone(), &token)
                .await
                .context("Error in service account login flow")?;

        self.event_handler
            .on_auth_success("Service account session created successfully")
            .await?;

        // Create a new PassClient (HACK to ensure store is fresh)
        let (client, _store) = self.create_client().await?;
        let pass_client = PassClient::new(client, client_features, AccountType::ServiceAccount);

        Ok((pass_client, service_account_key))
    }

    pub async fn complete_login(
        &self,
        pass_client: &PassClient,
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
