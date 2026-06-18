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

use crate::commands::item::agent_monitor::send_reason_if_agent;
use crate::commands::item::totp::generate_totp_token;
use crate::helpers::CliPassClient as PassClient;
use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use fluent_uri::Uri;
use pass::FindItemQuery;
use pass_domain::{EventAction, Field, ItemId, ShareId};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use urlencoding::decode;

/// Controls what is returned when resolving a TOTP field.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum TotpOutput {
    #[default]
    Code,
    Uri,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ItemReference {
    pub share_id: String,
    pub item_id: String,
    pub field_name: Option<String>,
    pub totp: Option<TotpOutput>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SecretReference {
    pub share_id: String,
    pub item_id: String,
    pub field_name: String,
    pub totp: Option<TotpOutput>,
}

impl ItemReference {
    pub fn parse(uri: &str) -> Result<Self> {
        // Percent-encode spaces; raw spaces are not valid URI characters,
        // but we allow them as a convenience.
        let normalized = uri.trim().replace(' ', "%20");

        // ensure_format borrows normalized and returns a Uri<&str> tied to it.
        let parsed = Self::ensure_format(&normalized)?;

        let auth = parsed.authority().ok_or_else(|| {
            anyhow!(
                "Invalid reference format. Expected: pass://SHARE_ID/ITEM_ID[/FIELD], got: [{}]",
                uri
            )
        })?;

        // host() returns the raw percent-encoded host string for any host variant.
        let host_enc = auth.host();
        if host_enc.is_empty() {
            return Err(anyhow!(
                "Invalid reference format. Expected: pass://SHARE_ID/ITEM_ID[/FIELD], got: [{}]",
                uri
            ));
        }
        let share_id = decode(host_enc)
            .with_context(|| format!("Failed to URL-decode share_id: {}", host_enc))?
            .into_owned();

        // segments_if_absolute() splits /item_id/field on '/' and yields EStr segments.
        let mut segments = parsed
            .path()
            .segments_if_absolute()
            .ok_or_else(|| {
                anyhow!(
                    "Cannot extract segments. Invalid reference format. Expected: pass://SHARE_ID/ITEM_ID[/FIELD], got: [{}]",
                    uri
                )
            })?;

        let item_id_seg = segments
            .next()
            .filter(|s| !s.as_str().is_empty())
            .ok_or_else(|| {
                anyhow!(
                    "Cannot extract item segment. Invalid reference format. Expected: pass://SHARE_ID/ITEM_ID[/FIELD], got: [{}]",
                    uri
                )
            })?;
        let item_id = item_id_seg
            .decode()
            .to_string()
            .map(|c| c.into_owned())
            .map_err(|_| anyhow!("item_id contains non-UTF-8 characters in: {}", uri))?;

        // Remaining segments form the field name (joined by '/' to preserve sub-paths).
        let field_parts: Vec<String> = segments
            .filter(|s| !s.as_str().is_empty())
            .map(|s| {
                s.decode()
                    .to_string()
                    .map(|c| c.into_owned())
                    .map_err(|_| anyhow!("field segment contains non-UTF-8 characters"))
            })
            .collect::<Result<_>>()?;
        let field_name = if field_parts.is_empty() {
            None
        } else {
            Some(field_parts.join("/"))
        };

        // ?totp=uri  -> return the raw otpauth:// URI stored in the field
        // ?totp=code -> return the computed TOTP code
        let totp_str = parsed.query().and_then(|q| {
            q.as_str().split('&').find_map(|pair| {
                let (key, value) = pair.split_once('=')?;
                if key == "totp" { Some(value) } else { None }
            })
        });
        let totp = match totp_str {
            Some("uri") => Some(TotpOutput::Uri),
            Some("code") => Some(TotpOutput::Code),
            Some("") | None => None,
            Some(other) => {
                return Err(anyhow!(
                    "Unknown totp output format '{}'. Expected 'uri' or 'code', in: {}",
                    other,
                    uri
                ));
            }
        };

        Ok(ItemReference {
            share_id,
            item_id,
            field_name,
            totp,
        })
    }

    // Takes a borrowed str so the returned Uri<&str> can borrow from it without
    // the ownership transfer that would drop the backing string too early.
    fn ensure_format(uri: &str) -> Result<Uri<&str>> {
        let parsed = Uri::parse(uri).map_err(|_| {
            anyhow!(
                "Invalid reference format. Expected: pass://SHARE_ID/ITEM_ID[/FIELD], got: [{}]",
                uri
            )
        })?;

        if parsed.scheme().as_str() != "pass" {
            return Err(anyhow!(
                "Invalid reference format. Expected: pass://SHARE_ID/ITEM_ID[/FIELD], got: [{}]",
                uri
            ));
        }

        if parsed.path().as_str().ends_with('/') {
            return Err(anyhow!(
                "Invalid reference format: trailing slash not allowed. Expected: pass://SHARE_ID/ITEM_ID[/FIELD], got: [{}]",
                uri
            ));
        }

        Ok(parsed)
    }
}

impl SecretReference {
    pub fn totp_output(&self) -> TotpOutput {
        self.totp.unwrap_or_default()
    }

    pub fn parse(uri: &str) -> Result<Self> {
        let item_ref = ItemReference::parse(uri)?;

        let field_name = item_ref.field_name
            .ok_or_else(|| anyhow!("Secret reference requires a field name. Expected: pass://SHARE_ID/ITEM_ID/FIELD_NAME, got: {}", uri))?;

        Ok(SecretReference {
            share_id: item_ref.share_id,
            item_id: item_ref.item_id,
            field_name,
            totp: item_ref.totp,
        })
    }
}

#[async_trait(?Send)]
pub trait SecretResolver {
    async fn resolve_secret(&self, secret_ref: &SecretReference) -> Result<String>;
    async fn resolve_secret_and_send_reason(&self, secret_ref: &SecretReference) -> Result<String>;
}

pub struct PassClientResolver {
    client: PassClient,
}

impl PassClientResolver {
    pub fn new(client: PassClient) -> Self {
        Self { client }
    }
}

#[async_trait(?Send)]
impl SecretResolver for PassClientResolver {
    async fn resolve_secret(&self, secret_ref: &SecretReference) -> Result<String> {
        let query = FindItemQuery::new(&secret_ref.share_id, &secret_ref.item_id);

        let share_id = ShareId::new(secret_ref.share_id.clone());
        let item_id = ItemId::new(secret_ref.item_id.clone());
        send_reason_if_agent(
            &self.client,
            EventAction::ItemRead,
            &share_id,
            Some(&item_id),
        )
        .await?;

        let item = self.client.find_item(query).await.with_context(|| {
            format!(
                "Failed to retrieve item {} from share {}",
                secret_ref.item_id, secret_ref.share_id
            )
        })?;

        let field = item.get_field(&secret_ref.field_name).ok_or_else(|| {
            anyhow!(
                "Field '{}' not found in item '{}'",
                secret_ref.field_name,
                secret_ref.item_id
            )
        })?;

        match field {
            Field::Totp(totp_uri) => match secret_ref.totp_output() {
                TotpOutput::Uri => Ok(totp_uri),
                TotpOutput::Code => generate_totp_token(&totp_uri),
            },
            _ => Ok(field.value()),
        }
    }
    async fn resolve_secret_and_send_reason(&self, secret_ref: &SecretReference) -> Result<String> {
        let share_id = ShareId::new(secret_ref.share_id.clone());
        let item_id = ItemId::new(secret_ref.item_id.clone());
        send_reason_if_agent(
            &self.client,
            EventAction::ItemRead,
            &share_id,
            Some(&item_id),
        )
        .await?;
        self.resolve_secret(secret_ref).await
    }
}

pub struct SecretCache {
    cache: Arc<Mutex<HashMap<String, String>>>,
}

impl SecretCache {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn get_or_resolve<R: SecretResolver>(
        &self,
        secret_ref: &SecretReference,
        resolver: &R,
    ) -> Result<String> {
        let totp_key = match secret_ref.totp {
            Some(TotpOutput::Uri) => "uri",
            Some(TotpOutput::Code) => "code",
            None => "",
        };
        let cache_key = format!(
            "{}:{}:{}:{}",
            secret_ref.share_id, secret_ref.item_id, secret_ref.field_name, totp_key
        );

        // Check cache first
        let mut cache = self.cache.lock().await;
        if let Some(cached_value) = cache.get(&cache_key) {
            return Ok(cached_value.clone());
        }

        // Drop the lock to make the async call
        drop(cache);

        // Fetch the secret
        let value = resolver
            .resolve_secret(secret_ref)
            .await
            .with_context(|| format!("Failed to fetch secret for {cache_key}"))?;

        // Re-acquire the lock and cache the value
        cache = self.cache.lock().await;
        cache.insert(cache_key, value.clone());
        Ok(value)
    }
}

/// Finds all pass:// URIs in the given text
pub fn find_pass_uri(text: &str) -> Option<String> {
    if text.starts_with("pass://") && ItemReference::parse(text).is_ok() {
        return Some(text.to_string());
    }

    None
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::collections::HashMap;

    pub(crate) struct StaticSecretResolver {
        pub(crate) secrets: HashMap<String, String>,
    }

    impl StaticSecretResolver {
        pub fn new() -> Self {
            Self {
                secrets: HashMap::new(),
            }
        }

        pub fn add_secret(
            &mut self,
            share_id: &str,
            item_id: &str,
            field_name: &str,
            value: &str,
        ) -> &mut Self {
            let key = format!("{share_id}:{item_id}:{field_name}");
            self.secrets.insert(key, value.to_string());
            self
        }

        pub fn with_secret(
            mut self,
            share_id: &str,
            item_id: &str,
            field_name: &str,
            value: &str,
        ) -> Self {
            let key = format!("{share_id}:{item_id}:{field_name}");
            self.secrets.insert(key, value.to_string());
            self
        }
    }

    #[async_trait(?Send)]
    impl SecretResolver for StaticSecretResolver {
        async fn resolve_secret(&self, secret_ref: &SecretReference) -> Result<String> {
            let key = format!(
                "{}:{}:{}",
                secret_ref.share_id, secret_ref.item_id, secret_ref.field_name
            );
            self.secrets.get(&key).cloned().ok_or_else(|| {
                anyhow!(
                    "Secret not found: {}:{}:{}",
                    secret_ref.share_id,
                    secret_ref.item_id,
                    secret_ref.field_name
                )
            })
        }

        async fn resolve_secret_and_send_reason(
            &self,
            secret_ref: &SecretReference,
        ) -> Result<String> {
            self.resolve_secret(secret_ref).await
        }
    }

    #[test]
    fn item_reference_parse_with_field() {
        let uri = "pass://share123/item456/password";
        let result = ItemReference::parse(uri).unwrap();

        assert_eq!(result.share_id, "share123");
        assert_eq!(result.item_id, "item456");
        assert_eq!(result.field_name, Some("password".to_string()));
    }

    #[test]
    fn item_reference_parse_without_field() {
        let uri = "pass://share123/item456";
        let result = ItemReference::parse(uri).unwrap();

        assert_eq!(result.share_id, "share123");
        assert_eq!(result.item_id, "item456");
        assert_eq!(result.field_name, None);
    }

    #[test]
    fn item_reference_parse_invalid() {
        let uri = "pass://share123";
        let result = ItemReference::parse(uri);
        assert!(result.is_err());

        let uri = "invalid://share123/item456/password";
        let result = ItemReference::parse(uri);
        assert!(result.is_err());
    }

    #[test]
    fn item_reference_parse_trailing_slash_invalid() {
        // Test trailing slash after item_id
        let uri = "pass://share123/item456/";
        let result = ItemReference::parse(uri);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("trailing slash not allowed")
        );

        // Test trailing slash after field
        let uri = "pass://share123/item456/password/";
        let result = ItemReference::parse(uri);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("trailing slash not allowed")
        );

        // Test trailing slash after share_id
        let uri = "pass://share123/";
        let result = ItemReference::parse(uri);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("trailing slash not allowed")
        );
    }

    #[test]
    fn item_reference_with_space_in_vault() {
        let uri = "pass://vault 123/item456";
        let result = ItemReference::parse(uri).expect("Failed to parse");
        assert_eq!(result.share_id, "vault 123");
        assert_eq!(result.item_id, "item456");
        assert_eq!(result.field_name, None);
    }

    #[test]
    fn item_reference_with_field_and_space_in_vault() {
        let uri = "pass://vault 123/item456/field";
        let result = ItemReference::parse(uri).expect("Failed to parse");
        assert_eq!(result.share_id, "vault 123");
        assert_eq!(result.item_id, "item456");
        assert_eq!(result.field_name, Some("field".to_string()));
    }

    #[test]
    fn item_reference_with_space_in_item() {
        let uri = "pass://vault123/item 456";
        let result = ItemReference::parse(uri).expect("Failed to parse");
        assert_eq!(result.share_id, "vault123");
        assert_eq!(result.item_id, "item 456");
        assert_eq!(result.field_name, None);
    }

    #[test]
    fn item_reference_with_field_and_space_in_item() {
        let uri = "pass://vault123/item 456/field";
        let result = ItemReference::parse(uri).expect("Failed to parse");
        assert_eq!(result.share_id, "vault123");
        assert_eq!(result.item_id, "item 456");
        assert_eq!(result.field_name, Some("field".to_string()));
    }

    #[test]
    fn item_reference_with_space_in_field() {
        let uri = "pass://vault123/item456/field number one";
        let result = ItemReference::parse(uri).expect("Failed to parse");
        assert_eq!(result.share_id, "vault123");
        assert_eq!(result.item_id, "item456");
        assert_eq!(result.field_name, Some("field number one".to_string()));
    }

    #[test]
    fn item_reference_with_spaces_in_all_components() {
        let uri = "pass://vault 123/item 456/field number one";
        let result = ItemReference::parse(uri).expect("Failed to parse");
        assert_eq!(result.share_id, "vault 123");
        assert_eq!(result.item_id, "item 456");
        assert_eq!(result.field_name, Some("field number one".to_string()));
    }

    #[test]
    fn secret_reference_parse_valid() {
        let uri = "pass://share123/item456/password";
        let result = SecretReference::parse(uri).unwrap();

        assert_eq!(result.share_id, "share123");
        assert_eq!(result.item_id, "item456");
        assert_eq!(result.field_name, "password");
    }

    #[test]
    fn secret_reference_parse_valid_complex_field() {
        let uri = "pass://share123/item456/custom_field_name";
        let result = SecretReference::parse(uri).unwrap();

        assert_eq!(result.share_id, "share123");
        assert_eq!(result.item_id, "item456");
        assert_eq!(result.field_name, "custom_field_name");
    }

    #[test]
    fn secret_reference_parse_requires_field() {
        let uri = "pass://share123/item456";
        let result = SecretReference::parse(uri);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("requires a field name")
        );
    }

    #[test]
    fn secret_reference_parse_invalid_missing_parts() {
        let invalid_uris = vec![
            "pass://share123",
            "pass://",
            "pass://share123/item456/", // trailing slash
            "share123/item456/password",
            "https://share123/item456/password",
        ];

        for uri in invalid_uris {
            assert!(
                SecretReference::parse(uri).is_err(),
                "Should fail for: {uri}"
            );
        }
    }

    #[test]
    fn secret_reference_parse_with_spaces() {
        let uri = "pass://My Vault/My Item/My Field";
        let result = SecretReference::parse(uri).unwrap();
        assert_eq!(result.share_id, "My Vault");
        assert_eq!(result.item_id, "My Item");
        assert_eq!(result.field_name, "My Field");
    }

    #[tokio::test]
    async fn secret_cache_get_or_resolve() {
        let resolver =
            StaticSecretResolver::new().with_secret("test", "item", "field", "cached_value");
        let cache = SecretCache::new();
        let secret_ref = SecretReference {
            share_id: "test".to_string(),
            item_id: "item".to_string(),
            field_name: "field".to_string(),
            totp: None,
        };

        // First call should resolve and cache
        let value1 = cache.get_or_resolve(&secret_ref, &resolver).await.unwrap();
        assert_eq!(value1, "cached_value");

        // Second call should use cache
        let value2 = cache.get_or_resolve(&secret_ref, &resolver).await.unwrap();
        assert_eq!(value2, "cached_value");
    }

    #[test]
    fn item_reference_parse_with_totp_code() {
        let uri = "pass://share123/item456/password?totp=code";
        let result = ItemReference::parse(uri).unwrap();
        assert_eq!(result.share_id, "share123");
        assert_eq!(result.item_id, "item456");
        assert_eq!(result.field_name, Some("password".to_string()));
        assert_eq!(result.totp, Some(TotpOutput::Code));
    }

    #[test]
    fn item_reference_parse_with_totp_uri() {
        let uri = "pass://share123/item456/password?totp=uri";
        let result = ItemReference::parse(uri).unwrap();
        assert_eq!(result.share_id, "share123");
        assert_eq!(result.item_id, "item456");
        assert_eq!(result.field_name, Some("password".to_string()));
        assert_eq!(result.totp, Some(TotpOutput::Uri));
    }

    #[test]
    fn item_reference_parse_without_totp() {
        let uri = "pass://share123/item456/password";
        let result = ItemReference::parse(uri).unwrap();
        assert_eq!(result.totp, None);
    }

    #[test]
    fn item_reference_parse_totp_ignores_other_params() {
        let uri = "pass://share123/item456/password?other=value&totp=code";
        let result = ItemReference::parse(uri).unwrap();
        assert_eq!(result.totp, Some(TotpOutput::Code));
    }

    #[test]
    fn item_reference_parse_empty_totp_treated_as_absent() {
        let uri = "pass://share123/item456/password?totp=";
        let result = ItemReference::parse(uri).unwrap();
        assert_eq!(result.totp, None);
    }

    #[test]
    fn item_reference_parse_unknown_totp_value_is_error() {
        let uri = "pass://share123/item456/password?totp=invalid";
        let result = ItemReference::parse(uri);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Unknown totp output format")
        );
    }

    #[test]
    fn secret_reference_parse_propagates_totp_uri() {
        let uri = "pass://share123/item456/password?totp=uri";
        let result = SecretReference::parse(uri).unwrap();
        assert_eq!(result.share_id, "share123");
        assert_eq!(result.item_id, "item456");
        assert_eq!(result.field_name, "password");
        assert_eq!(result.totp, Some(TotpOutput::Uri));
    }

    #[test]
    fn secret_reference_parse_propagates_totp_code() {
        let uri = "pass://share123/item456/password?totp=code";
        let result = SecretReference::parse(uri).unwrap();
        assert_eq!(result.totp, Some(TotpOutput::Code));
    }

    #[test]
    fn secret_reference_parse_no_totp() {
        let uri = "pass://share123/item456/password";
        let result = SecretReference::parse(uri).unwrap();
        assert_eq!(result.totp, None);
    }
}
