use super::cli_credential_provider::CliCredentialProvider;
use super::terminal_event_handler::TerminalEventHandler;
use crate::constants::SESSION_FILE_NAME;
use crate::features::CliClientFeatures;
use crate::storage::FileSystemSessionStorage;
use anyhow::Result;
use pass_auth::{Authenticator, ClientConfig};
use std::path::PathBuf;
use std::sync::Arc;

pub fn create_client_config() -> Result<ClientConfig> {
    let base_dir = crate::utils::get_base_dir()?;
    create_client_config_with_base_dir(base_dir)
}

pub fn create_client_config_with_base_dir(base_dir: PathBuf) -> Result<ClientConfig> {
    Ok(ClientConfig {
        base_dir,
        environment: std::env::var(pass_auth::ENVIRONMENT_ENV_VAR).ok(),
        proxy_config: pass_auth::ProxyConfig::from_env(),
        debug_config: pass_auth::config::DebugConfig::from_env(),
        app_header: None,
        post_login_config: pass_auth::PostLoginConfig::default(),
    })
}

pub fn create_authenticator(client_features: Arc<CliClientFeatures>) -> Result<Authenticator> {
    let config = create_client_config()?;
    create_authenticator_with_config(client_features, config)
}

#[allow(dead_code)]
pub fn create_authenticator_with_base_dir(
    client_features: Arc<CliClientFeatures>,
    base_dir: PathBuf,
) -> Result<Authenticator> {
    let config = create_client_config_with_base_dir(base_dir)?;
    create_authenticator_with_config(client_features, config)
}

pub fn create_authenticator_with_config(
    client_features: Arc<CliClientFeatures>,
    config: ClientConfig,
) -> Result<Authenticator> {
    let session_file_path = config.base_dir.join(SESSION_FILE_NAME);
    let storage = Arc::new(FileSystemSessionStorage::new(session_file_path));

    Ok(Authenticator::new(
        client_features.key_provider.clone(),
        storage,
        Arc::new(TerminalEventHandler),
        Arc::new(CliCredentialProvider),
        config,
    ))
}
