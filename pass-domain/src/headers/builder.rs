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

use super::detect_locale;
use crate::os_info::get_os_info;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ClientHeaders {
    pub accept_language: String,
    pub user_agent: String,
}

impl ClientHeaders {
    pub fn new(accept_language: String, user_agent: String) -> Self {
        Self {
            accept_language,
            user_agent,
        }
    }

    pub fn to_http_headers(&self) -> Vec<(String, String)> {
        vec![
            ("Accept-Language".into(), self.accept_language.clone()),
            ("User-Agent".into(), self.user_agent.clone()),
        ]
    }

    pub fn to_map(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.insert("Accept-Language".into(), self.accept_language.clone());
        map.insert("User-Agent".into(), self.user_agent.clone());
        map
    }
}

#[derive(Debug)]
pub struct HeaderBuilder {
    product_name: String,
    product_version: String,
    locale: Option<String>,
}

impl HeaderBuilder {
    pub fn new(product_name: &str, product_version: &str) -> Self {
        Self {
            product_name: product_name.into(),
            product_version: product_version.into(),
            locale: None,
        }
    }

    pub fn with_locale(mut self, locale: &str) -> Self {
        self.locale = Some(locale.into());
        self
    }

    pub fn build(self) -> ClientHeaders {
        let locale = self.locale.unwrap_or_else(detect_locale);
        ClientHeaders::new(
            build_accept_language(&locale),
            build_user_agent(&self.product_name, &self.product_version),
        )
    }
}

fn build_accept_language(locale: &str) -> String {
    let base_lang = locale.split('-').next().unwrap_or("en");
    format!("{},{};q=0.9", locale, base_lang)
}

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

fn build_user_agent(display_name: &str, product_version: &str) -> String {
    let os_info = get_os_info()
        .unwrap_or_else(|_| crate::os_info::OsInfo::new("Unknown".into(), "".into(), None));
    format!(
        "Proton{}/{} ({})",
        capitalize_first(display_name),
        product_version,
        os_info.user_agent_os_string()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_accept_language() {
        assert_eq!(build_accept_language("en-US"), "en-US,en;q=0.9");
        assert_eq!(build_accept_language("fr-FR"), "fr-FR,fr;q=0.9");
        assert_eq!(build_accept_language("de"), "de,de;q=0.9");
    }

    #[test]
    fn test_header_builder_basic() {
        let h = HeaderBuilder::new("cli-pass", "2.2.3").build();
        assert!(h.user_agent.starts_with("ProtonCli-pass/2.2.3"));
        assert!(h.accept_language.contains(','));
    }

    #[test]
    fn test_header_builder_with_locale() {
        let h = HeaderBuilder::new("cli-pass", "2.2.3")
            .with_locale("fr-FR")
            .build();
        assert_eq!(h.accept_language, "fr-FR,fr;q=0.9");
    }

    #[test]
    fn test_header_builder_chaining() {
        let h = HeaderBuilder::new("test-app", "1.0.0")
            .with_locale("de-DE")
            .build();
        assert_eq!(h.accept_language, "de-DE,de;q=0.9");
    }

    #[test]
    fn test_user_agent_format() {
        let h = HeaderBuilder::new("cli-pass", "2.2.3").build();
        assert!(h.user_agent.starts_with("Proton"));
        assert!(h.user_agent.contains("/"));
        assert!(h.user_agent.contains("(") && h.user_agent.contains(")"));
    }

    #[test]
    fn test_all_headers_present() {
        let h = HeaderBuilder::new("cli-pass", "2.2.3").build();
        assert!(!h.accept_language.is_empty());
        assert!(!h.user_agent.is_empty());
    }
}
