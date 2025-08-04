use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use pass::{FindItemQuery, PassClient};
use regex::Regex;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone, PartialEq)]
pub struct SecretReference {
    pub share_id: String,
    pub item_id: String,
    pub field_name: String,
}

impl SecretReference {
    pub fn parse(uri: &str) -> Result<Self> {
        let re = Regex::new(r"^pass://([^/]+)/([^/]+)/(.+)$")
            .map_err(|e| anyhow!("Failed to compile regex: {}", e))?;

        let captures = re.captures(uri)
            .ok_or_else(|| anyhow!("Invalid secret reference format. Expected: pass://SHARE_ID/ITEM_ID/FIELD_NAME, got: {}", uri))?;

        let share_id = captures
            .get(1)
            .ok_or_else(|| anyhow!("Missing share_id in secret reference"))?
            .as_str()
            .to_string();

        let item_id = captures
            .get(2)
            .ok_or_else(|| anyhow!("Missing item_id in secret reference"))?
            .as_str()
            .to_string();

        let field_name = captures
            .get(3)
            .ok_or_else(|| anyhow!("Missing field_name in secret reference"))?
            .as_str()
            .to_string();

        Ok(SecretReference {
            share_id,
            item_id,
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

        let field_value = item.get_field(&secret_ref.field_name).ok_or_else(|| {
            anyhow!(
                "Field '{}' not found in item '{}'",
                secret_ref.field_name,
                secret_ref.item_id
            )
        })?;

        Ok(field_value)
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
pub fn find_pass_uris(text: &str) -> Result<Vec<String>> {
    let re = Regex::new(r"pass://[^\s]+")
        .map_err(|e| anyhow!("Failed to compile pass URI regex: {}", e))?;

    let uris: Vec<String> = re.find_iter(text).map(|m| m.as_str().to_string()).collect();

    Ok(uris)
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
    fn secret_reference_parse_invalid_missing_parts() {
        let invalid_uris = vec![
            "pass://share123/item456",
            "pass://share123",
            "pass://",
            "pass://share123/item456/",
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

    #[test]
    fn find_pass_uris_in_text() {
        let text = "DB_PASSWORD=pass://prod/db/password API_KEY=pass://api/service/key";
        let uris = find_pass_uris(text).unwrap();

        assert_eq!(uris.len(), 2);
        assert_eq!(uris[0], "pass://prod/db/password");
        assert_eq!(uris[1], "pass://api/service/key");
    }

    #[test]
    fn find_pass_uris_no_matches() {
        let text = "DB_PASSWORD=secret123 API_KEY=abc123";
        let uris = find_pass_uris(text).unwrap();
        assert_eq!(uris.len(), 0);
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
