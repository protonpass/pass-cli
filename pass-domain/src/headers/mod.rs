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

mod builder;

pub use builder::{ClientHeaders, HeaderBuilder};

#[derive(Debug, Clone)]
pub struct HeaderError {
    pub message: String,
}

impl std::fmt::Display for HeaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for HeaderError {}

pub type HeaderResult<T> = Result<T, HeaderError>;

pub fn detect_locale() -> String {
    for var in &["LC_ALL", "LC_CTYPE", "LANG"] {
        if let Ok(locale) = std::env::var(var) {
            let normalized = normalize_locale(&locale);
            if !normalized.is_empty() {
                return normalized;
            }
        }
    }
    "en-US".into()
}

fn normalize_locale(locale: &str) -> String {
    let locale = locale.trim();
    if locale.is_empty() {
        return String::new();
    }
    let parts: Vec<&str> = locale.split(&['-', '_', '.'][..]).collect();
    if parts.is_empty() {
        return String::new();
    }
    let language = parts[0].to_lowercase();
    if parts.len() > 1 {
        format!("{}-{}", language, parts[1].to_uppercase())
    } else {
        // Default region for common languages
        match language.as_str() {
            "en" => "en-US",
            "fr" => "fr-FR",
            "de" => "de-DE",
            "es" => "es-ES",
            "it" => "it-IT",
            "pt" => "pt-PT",
            "nl" => "nl-NL",
            "pl" => "pl-PL",
            "ru" => "ru-RU",
            "ja" => "ja-JP",
            "zh" => "zh-CN",
            "ko" => "ko-KR",
            _ => return format!("{}-US", language),
        }
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_locale_us() {
        assert_eq!(normalize_locale("en-US"), "en-US");
    }
    #[test]
    fn test_normalize_locale_underscore() {
        assert_eq!(normalize_locale("en_US"), "en-US");
    }
    #[test]
    fn test_normalize_locale_encoding() {
        assert_eq!(normalize_locale("en_US.UTF-8"), "en-US");
    }
    #[test]
    fn test_normalize_locale_french() {
        assert_eq!(normalize_locale("fr_FR.UTF-8"), "fr-FR");
    }
    #[test]
    fn test_normalize_locale_german() {
        assert_eq!(normalize_locale("de-DE"), "de-DE");
    }
    #[test]
    fn test_normalize_locale_empty() {
        assert_eq!(normalize_locale(""), "");
    }
    #[test]
    fn test_normalize_locale_lang_only() {
        assert_eq!(normalize_locale("en"), "en-US");
        assert_eq!(normalize_locale("fr"), "fr-FR");
        assert_eq!(normalize_locale("de"), "de-DE");
        assert_eq!(normalize_locale("es"), "es-ES");
        assert_eq!(normalize_locale("unknown"), "unknown-US");
    }
    #[test]
    fn test_detect_locale() {
        let locale = detect_locale();
        assert!(!locale.is_empty());
        assert!(locale.contains('-'));
    }
}
