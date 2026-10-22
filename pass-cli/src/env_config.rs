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

pub const ENVIRONMENT_ENV_VAR: &str = "PROTON_PASS_ENVIRONMENT";
#[cfg(feature = "internal")]
pub const EXTRA_HEADERS_ENV_VAR: &str = "PROTON_PASS_CLI_EXTRA_HEADERS";

pub fn proxy_config_from_env() -> pass::auth::ProxyConfig {
    pass::auth::ProxyConfig::from_env()
}

pub fn debug_config_from_env() -> Option<pass::auth::DebugConfig> {
    std::env::var("XDEBUG_SESSION")
        .ok()
        .map(|session| pass::auth::DebugConfig {
            xdebug_session: Some(session),
        })
}

#[cfg(feature = "internal")]
pub fn extra_headers_from_env() -> Vec<pass::auth::ExtraHeader> {
    std::env::var(EXTRA_HEADERS_ENV_VAR)
        .map(|value| parse_extra_headers(&value))
        .unwrap_or_default()
}

#[cfg(not(feature = "internal"))]
pub fn extra_headers_from_env() -> Vec<pass::auth::ExtraHeader> {
    Vec::new()
}

#[cfg(feature = "internal")]
fn parse_extra_headers(value: &str) -> Vec<pass::auth::ExtraHeader> {
    value
        .split(';')
        .filter_map(|entry| {
            let (name, value) = entry.split_once('=')?;
            let name = name.trim();
            if name.is_empty() {
                return None;
            }
            Some(pass::auth::ExtraHeader {
                name: name.to_string(),
                value: value.trim().to_string(),
            })
        })
        .collect()
}

pub fn detect_locale() -> String {
    for var in &["LC_ALL", "LC_CTYPE", "LANG"] {
        if let Ok(locale) = std::env::var(var) {
            let normalized = pass::domain::headers::normalize_locale(&locale);
            if !normalized.is_empty() {
                return normalized;
            }
        }
    }
    pass::domain::headers::DEFAULT_LOCALE.to_string()
}

#[cfg(all(test, feature = "internal"))]
mod tests {
    use super::*;
    use pass::auth::ExtraHeader;

    fn header(name: &str, value: &str) -> ExtraHeader {
        ExtraHeader {
            name: name.to_string(),
            value: value.to_string(),
        }
    }

    #[test]
    fn parse_extra_headers_multiple() {
        assert_eq!(
            parse_extra_headers("header1=var1;header2=var2"),
            vec![header("header1", "var1"), header("header2", "var2")]
        );
    }

    #[test]
    fn parse_extra_headers_trims_and_skips_invalid() {
        assert_eq!(
            parse_extra_headers(" a = 1 ;;invalid;=novalue;b=;c=x=y;"),
            vec![header("a", "1"), header("b", ""), header("c", "x=y")]
        );
    }

    #[test]
    fn parse_extra_headers_empty() {
        assert!(parse_extra_headers("").is_empty());
    }
}
