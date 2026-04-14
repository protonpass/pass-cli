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
