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

use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct ClientConfig {
    pub base_dir: PathBuf,
    pub environment: Option<String>,
    pub proxy_config: ProxyConfig,
    pub debug_config: Option<DebugConfig>,
    pub(crate) app_header: String,
    pub(crate) child_client_id: String,
    pub post_login_config: PostLoginConfig,
    pub(crate) product_name: String,
    pub(crate) product_version: String,
    pub locale: Option<String>,
    pub extra_headers: Vec<ExtraHeader>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExtraHeader {
    pub name: String,
    pub value: String,
}

pub fn sanitize_app_name(name: &str) -> String {
    name.chars()
        .filter(char::is_ascii_alphabetic)
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

pub fn external_client_id(app_name: &str) -> anyhow::Result<String> {
    let sanitized = sanitize_app_name(app_name);
    if sanitized.is_empty() {
        anyhow::bail!("app name `{app_name}` must contain at least one ASCII letter");
    }
    Ok(format!("external-pass-{sanitized}"))
}

pub fn external_app_header(app_name: &str) -> anyhow::Result<String> {
    Ok(format!(
        "{}@{}",
        external_client_id(app_name)?,
        env!("CARGO_PKG_VERSION")
    ))
}

#[cfg(feature = "first-party")]
const FIRST_PARTY_CHILD_CLIENT_ID: &str = "cli-pass";

pub(crate) const HOST_LOOKUP_APP_NAME: &str = "external-pass-hostlookup";

impl ClientConfig {
    pub fn new(base_dir: PathBuf, app_name: &str) -> anyhow::Result<Self> {
        Ok(Self::build(
            base_dir,
            external_app_header(app_name)?,
            external_client_id(app_name)?,
            "Pass".to_string(),
            env!("CARGO_PKG_VERSION").to_string(),
        ))
    }

    #[cfg(feature = "first-party")]
    pub fn first_party(
        base_dir: PathBuf,
        app_header: String,
        product_name: String,
        product_version: String,
    ) -> Self {
        Self::build(
            base_dir,
            app_header,
            FIRST_PARTY_CHILD_CLIENT_ID.to_string(),
            product_name,
            product_version,
        )
    }

    pub fn app_header(&self) -> &str {
        &self.app_header
    }

    pub fn child_client_id(&self) -> &str {
        &self.child_client_id
    }

    pub fn sdk(&self) -> anyhow::Result<muon::common::sdk::Sdk> {
        let (name, version) = self
            .app_header
            .split_once('@')
            .ok_or_else(|| anyhow::anyhow!("app header `{}` has no version", self.app_header))?;
        muon::common::sdk::Sdk::new(name, version).map_err(|e| anyhow::anyhow!("{e}"))
    }

    fn build(
        base_dir: PathBuf,
        app_header: String,
        child_client_id: String,
        product_name: String,
        product_version: String,
    ) -> Self {
        Self {
            base_dir,
            environment: None,
            proxy_config: ProxyConfig::default(),
            debug_config: None,
            app_header,
            child_client_id,
            post_login_config: PostLoginConfig::default(),
            product_name,
            product_version,
            locale: None,
            extra_headers: Vec::new(),
        }
    }

    pub fn with_environment(mut self, env: String) -> Self {
        self.environment = Some(env);
        self
    }

    pub fn with_proxy_config(mut self, proxy_config: ProxyConfig) -> Self {
        self.proxy_config = proxy_config;
        self
    }

    pub fn with_debug_config(mut self, debug_config: DebugConfig) -> Self {
        self.debug_config = Some(debug_config);
        self
    }

    pub fn with_post_login_config(mut self, post_login_config: PostLoginConfig) -> Self {
        self.post_login_config = post_login_config;
        self
    }

    pub fn with_locale(mut self, locale: String) -> Self {
        self.locale = Some(locale);
        self
    }

    pub fn with_extra_headers(mut self, extra_headers: Vec<ExtraHeader>) -> Self {
        self.extra_headers = extra_headers;
        self
    }
}

#[derive(Clone, Debug, Default)]
pub struct ProxyConfig {
    pub http_proxy: Option<String>,
    pub https_proxy: Option<String>,
}

impl ProxyConfig {
    pub fn from_env() -> Self {
        Self {
            http_proxy: proxy_from_env("HTTP_PROXY"),
            https_proxy: proxy_from_env("HTTPS_PROXY"),
        }
    }
}

fn env_value(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}

fn proxy_from_env(upper: &str) -> Option<String> {
    env_value(upper).or_else(|| env_value(&upper.to_ascii_lowercase()))
}

#[cfg(tokio_runtime)]
pub(crate) fn proxy_env_var(upper: &str) -> String {
    proxy_env_var_with(upper, |name| env_value(name).is_some())
}

#[cfg(tokio_runtime)]
fn proxy_env_var_with(upper: &str, is_set: impl Fn(&str) -> bool) -> String {
    let lower = upper.to_ascii_lowercase();
    if !is_set(upper) && is_set(&lower) {
        lower
    } else {
        upper.to_string()
    }
}

#[derive(Clone, Debug)]
pub struct DebugConfig {
    pub xdebug_session: Option<String>,
}

#[derive(Clone, Debug)]
pub struct PostLoginConfig {
    pub create_default_vault: bool,
    pub default_vault_name: String,
}

impl Default for PostLoginConfig {
    fn default() -> Self {
        Self {
            create_default_vault: true,
            default_vault_name: "Personal".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_names() {
        assert_eq!(sanitize_app_name("My App"), "myapp");
        assert_eq!(sanitize_app_name("MyApP-123"), "myapp");
        assert_eq!(sanitize_app_name("Café"), "caf");
    }

    #[test]
    fn rejects_empty_names() {
        assert!(external_app_header("").is_err());
        assert!(external_app_header("123").is_err());
        assert!(external_app_header("---").is_err());
    }

    #[test]
    fn header_format_is_accepted_by_muon() {
        let header = external_app_header("My App").unwrap();
        assert_eq!(
            header,
            format!("external-pass-myapp@{}", env!("CARGO_PKG_VERSION"))
        );
        assert!(muon::App::new(&header).is_ok());
    }

    #[test]
    fn new_sets_header() {
        let config = ClientConfig::new(PathBuf::new(), "Foo").unwrap();
        assert_eq!(config.app_header(), external_app_header("Foo").unwrap());
        assert_eq!(config.product_name, "Pass");
    }

    #[test]
    fn sdk_takes_the_name_and_version_of_the_app_header() {
        let config = ClientConfig::new(PathBuf::new(), "My App").unwrap();
        let sdk = config.sdk().unwrap();
        assert_eq!(sdk.name(), "external-pass-myapp");
        assert_eq!(sdk.version(), env!("CARGO_PKG_VERSION"));
    }

    #[cfg(feature = "first-party")]
    #[test]
    fn app_header_without_version_has_no_sdk() {
        let config = ClientConfig::first_party(
            PathBuf::new(),
            "android-pass".to_string(),
            "Pass".to_string(),
            "1.0.0".to_string(),
        );
        assert!(config.sdk().is_err());
    }

    #[test]
    fn external_clients_log_in_with_their_own_client_id() {
        let config = ClientConfig::new(PathBuf::new(), "My App").unwrap();
        assert_eq!(config.child_client_id(), "external-pass-myapp");
    }

    #[cfg(feature = "first-party")]
    #[test]
    fn first_party_keeps_the_cli_client_id() {
        let config = ClientConfig::first_party(
            PathBuf::new(),
            "cli-pass@1.0.0".to_string(),
            "Pass".to_string(),
            "1.0.0".to_string(),
        );
        assert_eq!(config.child_client_id(), "cli-pass");
    }

    #[test]
    fn host_lookup_app_name_resolves_to_the_pass_api() {
        use muon::app::{AppName, AppVersion, SemVer};
        use muon::env::{Env, Environment};
        use std::str::FromStr;

        let version = AppVersion::Named {
            name: AppName::from_str(HOST_LOOKUP_APP_NAME).unwrap(),
            version: SemVer::from_str("1.0.0").unwrap(),
        };
        let servers = Environment::new_atlas().servers(&version);
        let host = format!("{}", servers[0].host().name());
        assert_eq!(host, "pass-api.proton.black");
    }
}
