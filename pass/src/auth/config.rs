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
    /// Sent as `x-pm-appversion`, in muon's `<platform>-<product>@<version>`
    /// format (e.g. `cli-pass@1.2.3`). Must be a client/version the backend
    /// accepts, so it's always up to the consumer to provide it.
    pub app_header: String,
    pub post_login_config: PostLoginConfig,
    /// Display name used in the User-Agent (e.g. `Pass`).
    pub product_name: String,
    /// Consumer's version, used in the User-Agent.
    pub product_version: String,
    pub locale: Option<String>,
    pub extra_headers: Vec<ExtraHeader>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExtraHeader {
    pub name: String,
    pub value: String,
}

impl ClientConfig {
    pub fn new(
        base_dir: PathBuf,
        app_header: String,
        product_name: String,
        product_version: String,
    ) -> Self {
        Self {
            base_dir,
            environment: None,
            proxy_config: ProxyConfig::default(),
            debug_config: None,
            app_header,
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
