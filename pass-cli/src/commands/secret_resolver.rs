use crate::helpers::CliPassClient as PassClient;
use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use pass::FindItemQuery;
use regex::Regex;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use urlencoding::decode;

#[derive(Debug, Clone, PartialEq)]
pub struct ItemReference {
    pub share_id: String,
    pub item_id: String,
    pub field_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SecretReference {
    pub share_id: String,
    pub item_id: String,
    pub field_name: String,
}

impl ItemReference {
    pub fn parse(uri: &str) -> Result<Self> {
        // Check for trailing slash first - this is invalid
        if uri.ends_with('/') {
            return Err(anyhow!(
                "Invalid reference format: trailing slash not allowed. Expected: pass://SHARE_ID/ITEM_ID or pass://SHARE_ID/ITEM_ID/FIELD, got: {}",
                uri
            ));
        }

        // Try to parse with field first (3 parts) - field must be non-empty
        let re_with_field = Regex::new(r"^pass://([^/]+)/([^/]+)/(.+)$")
            .map_err(|e| anyhow!("Failed to compile regex: {}", e))?;

        if let Some(captures) = re_with_field.captures(uri) {
            let share_id = captures
                .get(1)
                .ok_or_else(|| anyhow!("Missing share_id in reference"))?
                .as_str();

            let item_id = captures
                .get(2)
                .ok_or_else(|| anyhow!("Missing item_id in reference"))?
                .as_str();

            let field_name = captures.get(3).map(|m| m.as_str());

            // URL-decode the components
            let share_id = decode(share_id)
                .with_context(|| format!("Failed to URL-decode share_id: {}", share_id))?
                .to_string();
            let item_id = decode(item_id)
                .with_context(|| format!("Failed to URL-decode item_id: {}", item_id))?
                .to_string();
            let field_name = field_name
                .map(|f| {
                    decode(f)
                        .with_context(|| format!("Failed to URL-decode field_name: {}", f))
                        .map(|s| s.to_string())
                })
                .transpose()?;

            return Ok(ItemReference {
                share_id,
                item_id,
                field_name,
            });
        }

        // Try to parse without field (2 parts)
        let re_without_field = Regex::new(r"^pass://([^/]+)/([^/]+)$")
            .map_err(|e| anyhow!("Failed to compile regex: {}", e))?;

        if let Some(captures) = re_without_field.captures(uri) {
            let share_id = captures
                .get(1)
                .ok_or_else(|| anyhow!("Missing share_id in reference"))?
                .as_str();

            let item_id = captures
                .get(2)
                .ok_or_else(|| anyhow!("Missing item_id in reference"))?
                .as_str();

            // URL-decode the components
            let share_id = decode(share_id)
                .with_context(|| format!("Failed to URL-decode share_id: {}", share_id))?
                .to_string();
            let item_id = decode(item_id)
                .with_context(|| format!("Failed to URL-decode item_id: {}", item_id))?
                .to_string();

            return Ok(ItemReference {
                share_id,
                item_id,
                field_name: None,
            });
        }

        Err(anyhow!(
            "Invalid reference format. Expected: pass://SHARE_ID/ITEM_ID or pass://SHARE_ID/ITEM_ID/FIELD, got: {}",
            uri
        ))
    }
}

impl SecretReference {
    pub fn parse(uri: &str) -> Result<Self> {
        let item_ref = ItemReference::parse(uri)?;

        let field_name = item_ref.field_name
            .ok_or_else(|| anyhow!("Secret reference requires a field name. Expected: pass://SHARE_ID/ITEM_ID/FIELD_NAME, got: {}", uri))?;

        Ok(SecretReference {
            share_id: item_ref.share_id,
            item_id: item_ref.item_id,
            field_name,
        })
    }
}

#[async_trait(?Send)]
pub trait SecretResolver {
    async fn resolve_secret(&self, secret_ref: &SecretReference) -> Result<String>;
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

        Ok(field.value())
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
        let cache_key = format!(
            "{}:{}:{}",
            secret_ref.share_id, secret_ref.item_id, secret_ref.field_name
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
        };

        // First call should resolve and cache
        let value1 = cache.get_or_resolve(&secret_ref, &resolver).await.unwrap();
        assert_eq!(value1, "cached_value");

        // Second call should use cache
        let value2 = cache.get_or_resolve(&secret_ref, &resolver).await.unwrap();
        assert_eq!(value2, "cached_value");
    }
}
