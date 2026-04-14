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

use crate::constants::SESSION_FILE_NAME;
use crate::features::CliClientFeatures;
use crate::storage::FileSystemSessionStorage;
use crate::utils::ask_for_input;
use anyhow::Context;
use muon::env::Environment;
use pass_auth::os::ProdClient;
use pass_auth::store::{CustomEnv, PassSessionStore, SerializedEnv};
use std::io::Read;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

const APP_NAME: &str = "cli-pass";

const PASSWORD_ENV_VAR: &str = "PROTON_PASS_PASSWORD";
const PASSWORD_FILE_ENV_VAR: &str = "PROTON_PASS_PASSWORD_FILE";
const EXTRA_PASSWORD_ENV_VAR: &str = "PROTON_PASS_EXTRA_PASSWORD";
const EXTRA_PASSWORD_FILE_ENV_VAR: &str = "PROTON_PASS_EXTRA_PASSWORD_FILE";
const TOTP_ENV_VAR: &str = "PROTON_PASS_TOTP";
const TOTP_FILE_ENV_VAR: &str = "PROTON_PASS_TOTP_FILE";
const USERNAME_ENV_VAR: &str = "PROTON_PASS_USERNAME";
const USERNAME_FILE_ENV_VAR: &str = "PROTON_PASS_USERNAME_FILE";
const APP_HEADER_ENV_VAR: &str = "PROTON_PASS_APP_HEADER";

fn get_env() -> SerializedEnv {
    match std::env::var(pass_auth::ENVIRONMENT_ENV_VAR) {
        Ok(v) => {
            if v == "atlas" {
                SerializedEnv::Atlas(None)
            } else if v == "localhost" {
                SerializedEnv::Custom(CustomEnv::Localhost)
            } else if v.starts_with("http") {
                SerializedEnv::Custom(CustomEnv::CustomUrl(v))
            } else {
                SerializedEnv::Atlas(Some(v))
            }
        }
        Err(_) => SerializedEnv::Prod,
    }
}

pub fn get_value(
    env_var: &str,
    file_env_var: &str,
    prompt: &str,
    secure: bool,
) -> anyhow::Result<String> {
    match std::env::var(env_var) {
        Ok(v) => Ok(v),
        Err(_) => match std::env::var(file_env_var) {
            Ok(v) => {
                let mut f = std::fs::File::open(v).context("Error opening file")?;
                let mut buff = String::new();
                f.read_to_string(&mut buff).context("Error reading file")?;
                Ok(buff.trim().to_string())
            }
            Err(_) => ask_for_input(prompt, secure),
        },
    }
}

pub fn get_password() -> anyhow::Result<String> {
    get_value(
        PASSWORD_ENV_VAR,
        PASSWORD_FILE_ENV_VAR,
        "Enter password: ",
        true,
    )
}

pub fn get_extra_password() -> anyhow::Result<String> {
    get_value(
        EXTRA_PASSWORD_ENV_VAR,
        EXTRA_PASSWORD_FILE_ENV_VAR,
        "Enter Pass extra password: ",
        true,
    )
}

pub fn get_totp() -> anyhow::Result<String> {
    get_value(TOTP_ENV_VAR, TOTP_FILE_ENV_VAR, "Enter TOTP: ", false)
}

pub fn get_username() -> anyhow::Result<String> {
    get_value(
        USERNAME_ENV_VAR,
        USERNAME_FILE_ENV_VAR,
        "Enter username: ",
        false,
    )
}

fn default_app_header() -> String {
    format!("{}@{}", APP_NAME, env!("CARGO_PKG_VERSION"))
}

fn get_app_header() -> String {
    std::env::var(APP_HEADER_ENV_VAR).unwrap_or_else(|_| default_app_header())
}

fn store_using_current_env(env: &Environment) -> bool {
    let current_env = Environment::from(get_env());
    match (&current_env, env) {
        (Environment::Prod(_), Environment::Prod(_)) => true,
        (Environment::Custom(_), Environment::Custom(_)) => true,
        (Environment::Atlas(_), Environment::Atlas(_)) => true,
        (Environment::Scientist(_), Environment::Scientist(_)) => {
            // Compare serialized forms
            let cur_ser = SerializedEnv::from(current_env.clone());
            let env_ser = SerializedEnv::from(env.clone());
            matches!(
                (cur_ser, env_ser),
                (SerializedEnv::Atlas(Some(a)), SerializedEnv::Atlas(Some(b))) if a == b
            )
        }
        _ => false,
    }
}

pub async fn get_client(
    base_dir: PathBuf,
    client_features: Arc<CliClientFeatures>,
) -> anyhow::Result<(ProdClient, Arc<RwLock<PassSessionStore>>)> {
    let session_file_path = base_dir.join(SESSION_FILE_NAME);
    let storage = Arc::new(FileSystemSessionStorage::new(session_file_path));

    let config = pass_auth::ClientConfig {
        base_dir: base_dir.clone(),
        environment: std::env::var(pass_auth::ENVIRONMENT_ENV_VAR).ok(),
        proxy_config: pass_auth::ProxyConfig::from_env(),
        debug_config: pass_auth::config::DebugConfig::from_env(),
        app_header: Some(get_app_header()),
        post_login_config: pass_auth::PostLoginConfig::default(),
    };

    let result = pass_auth::client_builder::create_client(
        client_features.key_provider.clone(),
        storage,
        &config,
    )
    .await;

    match result {
        Ok((client, store)) => {
            // Check if environment has switched
            let env_switched = {
                let store_guard = store.read().expect("store rwlock poisoned");
                !store_using_current_env(&store_guard.env)
            };
            if env_switched {
                eprintln!(
                    "PROTON_PASS_ENVIRONMENT has switched! Logging you out. Please log back in again"
                );
                crate::commands::logout::force_logout().await?;
                std::process::exit(1);
            }
            Ok((client, store))
        }
        Err(e) => Err(e),
    }
}
