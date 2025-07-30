use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use pass::{FindItemQuery, PassClient};
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::io::{self, Read, Write};
use std::sync::Arc;
use tokio::sync::Mutex;

fn compile_pass_uri_regex() -> Result<Regex> {
    Regex::new(r"\{\{\s*(pass://[^}]+)\s*\}\}")
        .map_err(|e| anyhow!("Failed to compile pass URI regex: {}", e))
}

fn set_file_permissions(file_path: &str, mode: u32) -> Result<()> {
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    {
        let file = fs::File::open(file_path).with_context(|| {
            format!("Failed to open output file for permission setting: {file_path}")
        })?;

        let mut permissions = file
            .metadata()
            .context("Failed to get file metadata")?
            .permissions();

        permissions.set_mode(mode);
        fs::set_permissions(file_path, permissions)
            .with_context(|| format!("Failed to set file permissions: {file_path}"))?;
    }

    #[cfg(not(unix))]
    {
        // On non-Unix systems (like Windows), we can't set Unix-style permissions
        // The file will use default permissions for the platform
        let _ = (file_path, mode); // Suppress unused parameter warnings
    }

    Ok(())
}

pub async fn run(
    file_mode: String,
    force: bool,
    in_file: Option<String>,
    out_file: Option<String>,
    client: PassClient,
) -> Result<()> {
    // Read input template
    let template = match in_file {
        Some(file_path) => fs::read_to_string(&file_path)
            .with_context(|| format!("Failed to read input file: {file_path}"))?,
        None => {
            let mut buffer = String::new();
            io::stdin()
                .read_to_string(&mut buffer)
                .context("Failed to read from stdin")?;
            buffer
        }
    };

    // Process the template
    let resolver = PassClientResolver::new(client);
    let processor = TemplateProcessor::new(resolver);
    let processed_content = processor
        .process_template(&template)
        .await
        .context("Failed to process template")?;

    // Write output
    match out_file {
        Some(output_path) => {
            // Check if file exists and prompt for confirmation if needed
            if std::path::Path::new(&output_path).exists() && !force {
                eprintln!("File '{output_path}' already exists. Use --force to overwrite.");
                return Err(anyhow!("Output file already exists"));
            }

            // Write to file
            fs::write(&output_path, processed_content)
                .with_context(|| format!("Failed to write to output file: {output_path}"))?;

            // Set file permissions
            let mode = parse_file_mode(&file_mode)
                .with_context(|| format!("Invalid file mode: {file_mode}"))?;

            set_file_permissions(&output_path, mode)?;

            eprintln!("Secrets injected successfully to: {output_path}");
        }
        None => {
            // Write to stdout
            print!("{processed_content}");
            io::stdout().flush().context("Failed to flush stdout")?;
        }
    }

    Ok(())
}

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

pub struct TemplateProcessor<R: SecretResolver> {
    resolver: R,
    secrets_cache: Arc<Mutex<HashMap<String, String>>>,
}

impl<R: SecretResolver> TemplateProcessor<R> {
    pub fn new(resolver: R) -> Self {
        Self {
            resolver,
            secrets_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn process_template(&self, template: &str) -> Result<String> {
        // Now use regex to find pass:// URIs within handlebars expressions
        // This regex ensures we only match URIs that are within {{ }} blocks
        let pass_ref_re = compile_pass_uri_regex()?;

        let mut result = template.to_string();
        let mut cache = self.secrets_cache.lock().await;

        for captures in pass_ref_re.captures_iter(template) {
            let full_match = captures.get(0).unwrap().as_str();
            let uri = captures.get(1).unwrap().as_str().trim();

            // Parse the URI
            let secret_ref = SecretReference::parse(uri)
                .with_context(|| format!("Invalid secret reference: {uri}"))?;

            // Create cache key
            let cache_key = format!(
                "{}:{}:{}",
                secret_ref.share_id, secret_ref.item_id, secret_ref.field_name
            );

            // Check cache first
            let secret_value = if let Some(cached_value) = cache.get(&cache_key) {
                cached_value.clone()
            } else {
                // We need to drop the lock to make the async call
                drop(cache);

                // Fetch the secret
                let value = self
                    .resolver
                    .resolve_secret(&secret_ref)
                    .await
                    .with_context(|| format!("Failed to fetch secret for {uri}"))?;

                // Re-acquire the lock and cache the value
                cache = self.secrets_cache.lock().await;
                cache.insert(cache_key, value.clone());
                value
            };

            // Replace the entire handlebars expression with the secret value
            result = result.replace(full_match, &secret_value);
        }

        Ok(result)
    }
}

/// Parses a file mode string into a u32 value
/// Supports both octal (0600) and decimal (600) formats
/// Note: File modes are only meaningful on Unix systems
fn parse_file_mode(mode_str: &str) -> Result<u32> {
    if mode_str.starts_with('0') {
        // Octal format
        u32::from_str_radix(mode_str, 8)
            .map_err(|e| anyhow!("Invalid octal file mode '{}': {}", mode_str, e))
    } else {
        // Decimal format
        mode_str
            .parse::<u32>()
            .map_err(|e| anyhow!("Invalid file mode '{}': {}", mode_str, e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct StaticSecretResolver {
        secrets: HashMap<String, String>,
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
            self.add_secret(share_id, item_id, field_name, value);
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
    fn parse_file_mode_octal() {
        assert_eq!(parse_file_mode("0600").unwrap(), 0o600);
        assert_eq!(parse_file_mode("0644").unwrap(), 0o644);
        assert_eq!(parse_file_mode("0755").unwrap(), 0o755);
    }

    #[test]
    fn parse_file_mode_decimal() {
        assert_eq!(parse_file_mode("600").unwrap(), 600);
        assert_eq!(parse_file_mode("644").unwrap(), 644);
    }

    #[test]
    fn parse_file_mode_invalid() {
        assert!(parse_file_mode("invalid").is_err());
        assert!(parse_file_mode("0888").is_err()); // Invalid octal
    }

    #[test]
    fn template_processing_pattern_recognition() {
        // Test that our regex correctly identifies secret references within handlebars expressions
        let re = compile_pass_uri_regex().unwrap();

        let test_cases = vec![
            ("{{ pass://share/item/field }}", true),
            ("{{pass://share/item/field}}", true),
            ("{{ pass://share123/item456/password }}", true),
            ("{{ pass://share/item/field_with_underscores }}", true),
            ("text before {{ pass://share/item/field }} text after", true),
            ("{{ op://share/item/field }}", false),
            ("{{ pass://share/item }}", true), // This would be caught but would fail URI parsing
            ("pass://share/item/field", false), // Not in template block - should not match!
            // Test cases with spaces
            ("{{ pass://My Vault/item/field }}", true),
            ("{{ pass://vault/My Item/field }}", true),
            ("{{ pass://vault/item/My Field }}", true),
            ("{{ pass://My Vault/My Item/My Field }}", true),
            ("{{pass://My Vault/My Item/My Field}}", true), // No spaces around template
            (
                "text {{ pass://My Vault/My Item/password }} more text",
                true,
            ),
        ];

        for (input, should_match) in test_cases {
            let matches = re.is_match(input);
            assert_eq!(matches, should_match, "Failed for input: {input}");
        }
    }

    #[test]
    fn template_ignores_non_template_pass_uris() {
        // Test that pass:// URIs outside of template blocks are ignored
        let template = r#"
# This is a config file
# Documentation: see pass://company-wiki/docs/setup for details

database:
  # The password is stored at pass://prod-db/main/password (not templated)
  password: {{ pass://prod-db/main/password }}
  
# Another comment with pass://other/item/field should be ignored
api_key: {{ pass://api/service/key }}
"#;

        let re = compile_pass_uri_regex().unwrap();
        let matches: Vec<_> = re.captures_iter(template).collect();

        // Should only match the two templated references, not the ones in comments
        assert_eq!(matches.len(), 2);

        // Check the captured URIs (trimmed, as they would be used in processing)
        assert_eq!(
            matches[0].get(1).unwrap().as_str().trim(),
            "pass://prod-db/main/password"
        );
        assert_eq!(
            matches[1].get(1).unwrap().as_str().trim(),
            "pass://api/service/key"
        );
    }

    #[test]
    fn template_processing_with_spaces() {
        // Test template processing with URIs containing spaces
        let template = r#"
# Configuration for My Application
database:
  host: {{ pass://My Production Vault/Database Server/hostname }}
  username: {{ pass://My Production Vault/Database Server/username }}
  password: {{ pass://My Production Vault/Database Server/password }}

api:
  # External service credentials  
  key: {{ pass://API Keys/External Service/api key }}
  secret: {{ pass://API Keys/External Service/secret token }}
"#;

        let re = compile_pass_uri_regex().unwrap();
        let matches: Vec<_> = re.captures_iter(template).collect();

        // Should match 5 different secret references with spaces
        assert_eq!(matches.len(), 5);

        // Check that all references with spaces are properly captured
        let expected_uris = [
            "pass://My Production Vault/Database Server/hostname",
            "pass://My Production Vault/Database Server/username",
            "pass://My Production Vault/Database Server/password",
            "pass://API Keys/External Service/api key",
            "pass://API Keys/External Service/secret token",
        ];

        for (i, captures) in matches.iter().enumerate() {
            let captured_uri = captures.get(1).unwrap().as_str().trim();
            assert_eq!(captured_uri, expected_uris[i]);

            // Also verify that each URI can be parsed correctly
            let parsed = SecretReference::parse(captured_uri);
            assert!(parsed.is_ok(), "Failed to parse URI: {captured_uri}");
        }

        // Verify specific parsing results
        let first_ref = SecretReference::parse(expected_uris[0]).unwrap();
        assert_eq!(first_ref.share_id, "My Production Vault");
        assert_eq!(first_ref.item_id, "Database Server");
        assert_eq!(first_ref.field_name, "hostname");

        let last_ref = SecretReference::parse(expected_uris[4]).unwrap();
        assert_eq!(last_ref.share_id, "API Keys");
        assert_eq!(last_ref.item_id, "External Service");
        assert_eq!(last_ref.field_name, "secret token");
    }

    #[test]
    fn template_processing_multiple_references() {
        // Test template with multiple secret references
        let template = r#"
database:
  host: {{ pass://infra-prod/db-server/host }}
  port: {{ pass://infra-prod/db-server/port }}
  user: {{ pass://infra-prod/db-creds/username }}
  password: {{ pass://infra-prod/db-creds/password }}

api:
  key: {{ pass://api-keys/external-service/api_key }}
  secret: {{ pass://api-keys/external-service/secret }}
"#;

        let re = compile_pass_uri_regex().unwrap();
        let matches: Vec<_> = re.captures_iter(template).collect();

        // Should match 6 different secret references
        assert_eq!(matches.len(), 6);

        // Check that all references are properly captured as full URIs
        let expected_uris = [
            "pass://infra-prod/db-server/host",
            "pass://infra-prod/db-server/port",
            "pass://infra-prod/db-creds/username",
            "pass://infra-prod/db-creds/password",
            "pass://api-keys/external-service/api_key",
            "pass://api-keys/external-service/secret",
        ];

        for (i, captures) in matches.iter().enumerate() {
            let captured_uri = captures.get(1).unwrap().as_str().trim();
            assert_eq!(captured_uri, expected_uris[i]);

            // Also verify that each URI can be parsed correctly
            let parsed = SecretReference::parse(captured_uri);
            assert!(parsed.is_ok(), "Failed to parse URI: {captured_uri}");
        }
    }

    #[test]
    fn template_with_no_references() {
        let template = r#"
server:
  host: localhost
  port: 8080
  name: test-server
"#;

        let re = compile_pass_uri_regex().unwrap();
        let matches: Vec<_> = re.captures_iter(template).collect();

        // Should have no matches
        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn template_with_mixed_content() {
        let template = r#"
# Configuration file
server_url: https://example.com
admin_password: {{ pass://admin/credentials/password }}
debug: true
database_url: postgres://user:{{ pass://db/main/password }}@localhost:5432/mydb
"#;

        let re = compile_pass_uri_regex().unwrap();
        let matches: Vec<_> = re.captures_iter(template).collect();

        // Should match 2 references
        assert_eq!(matches.len(), 2);

        // Verify the full URIs are captured (trimmed, as they would be used in processing)
        assert_eq!(
            matches[0].get(1).unwrap().as_str().trim(),
            "pass://admin/credentials/password"
        );
        assert_eq!(
            matches[1].get(1).unwrap().as_str().trim(),
            "pass://db/main/password"
        );

        // Verify they can be parsed correctly
        let ref1 = SecretReference::parse(matches[0].get(1).unwrap().as_str().trim()).unwrap();
        assert_eq!(ref1.share_id, "admin");
        assert_eq!(ref1.item_id, "credentials");
        assert_eq!(ref1.field_name, "password");

        let ref2 = SecretReference::parse(matches[1].get(1).unwrap().as_str().trim()).unwrap();
        assert_eq!(ref2.share_id, "db");
        assert_eq!(ref2.item_id, "main");
        assert_eq!(ref2.field_name, "password");
    }

    #[test]
    fn secret_reference_with_special_characters() {
        // Test with field names that have special characters (which are valid)
        let uri = "pass://share-prod-123/item_456/field-name_with.special-chars";
        let result = SecretReference::parse(uri).unwrap();

        assert_eq!(result.share_id, "share-prod-123");
        assert_eq!(result.item_id, "item_456");
        assert_eq!(result.field_name, "field-name_with.special-chars");
    }

    #[test]
    fn secret_reference_with_spaces() {
        // Test with spaces in share_id
        let uri1 = "pass://My Vault/item456/password";
        let result1 = SecretReference::parse(uri1).unwrap();
        assert_eq!(result1.share_id, "My Vault");
        assert_eq!(result1.item_id, "item456");
        assert_eq!(result1.field_name, "password");

        // Test with spaces in item_id
        let uri2 = "pass://vault123/My Login Item/password";
        let result2 = SecretReference::parse(uri2).unwrap();
        assert_eq!(result2.share_id, "vault123");
        assert_eq!(result2.item_id, "My Login Item");
        assert_eq!(result2.field_name, "password");

        // Test with spaces in field_name
        let uri3 = "pass://vault123/item456/My Custom Field";
        let result3 = SecretReference::parse(uri3).unwrap();
        assert_eq!(result3.share_id, "vault123");
        assert_eq!(result3.item_id, "item456");
        assert_eq!(result3.field_name, "My Custom Field");

        // Test with spaces in all parts
        let uri4 = "pass://My Personal Vault/My Login Item/My Custom Field";
        let result4 = SecretReference::parse(uri4).unwrap();
        assert_eq!(result4.share_id, "My Personal Vault");
        assert_eq!(result4.item_id, "My Login Item");
        assert_eq!(result4.field_name, "My Custom Field");
    }

    #[tokio::test]
    async fn template_processor_with_static_resolver() {
        let mut resolver = StaticSecretResolver::new();
        resolver
            .add_secret("prod-db", "main", "password", "supersecret123")
            .add_secret("prod-db", "main", "username", "dbuser")
            .add_secret("api-keys", "stripe", "secret_key", "sk_live_abcd1234");

        let processor = TemplateProcessor::new(resolver);

        let template = r#"
database:
  host: localhost
  username: {{ pass://prod-db/main/username }}
  password: {{ pass://prod-db/main/password }}

api:
  stripe_key: {{ pass://api-keys/stripe/secret_key }}
"#;

        let result = processor.process_template(template).await.unwrap();

        assert!(result.contains("username: dbuser"));
        assert!(result.contains("password: supersecret123"));
        assert!(result.contains("stripe_key: sk_live_abcd1234"));
        assert!(result.contains("host: localhost")); // Non-template content preserved
    }

    #[tokio::test]
    async fn template_processor_with_spaces_in_uris() {
        let mut resolver = StaticSecretResolver::new();
        resolver
            .add_secret(
                "My Vault",
                "Database Server",
                "password",
                "space_password123",
            )
            .add_secret("My Vault", "Database Server", "username", "space_user")
            .add_secret(
                "API Keys",
                "External Service",
                "api key",
                "space_api_key_456",
            );

        let processor = TemplateProcessor::new(resolver);

        let template = r#"
database:
  host: localhost
  username: {{ pass://My Vault/Database Server/username }}
  password: {{ pass://My Vault/Database Server/password }}

api:
  key: {{ pass://API Keys/External Service/api key }}
"#;

        let result = processor.process_template(template).await.unwrap();

        assert!(result.contains("username: space_user"));
        assert!(result.contains("password: space_password123"));
        assert!(result.contains("key: space_api_key_456"));
        assert!(result.contains("host: localhost")); // Non-template content preserved
    }

    #[tokio::test]
    async fn template_processor_multiple_occurences() {
        let resolver =
            StaticSecretResolver::new().with_secret("test", "item", "field", "same_value");
        let processor = TemplateProcessor::new(resolver);

        let template = r#"
first: {{ pass://test/item/field }}
second: {{ pass://test/item/field }}
"#;

        let result = processor.process_template(template).await.unwrap();

        // Both references should have the same content
        assert!(result.contains("first: same_value"));
        assert!(result.contains("second: same_value"));
    }

    #[tokio::test]
    async fn template_processor_error_handling() {
        let resolver = StaticSecretResolver::new();
        let processor = TemplateProcessor::new(resolver);

        let template = "secret: {{ pass://nonexistent/item/field }}";

        let result = processor.process_template(template).await;
        assert!(result.is_err());

        let source = result.unwrap_err().source().unwrap().to_string();
        assert!(source.contains("Secret not found"));
    }

    #[tokio::test]
    async fn template_processor_ignores_non_template_uris() {
        let resolver =
            StaticSecretResolver::new().with_secret("real", "secret", "password", "should_replace");
        let processor = TemplateProcessor::new(resolver);

        let template = r#"
# Documentation: see pass://docs/wiki/setup for details
# The password is stored at pass://real/secret/password (not templated)
actual_password: {{ pass://real/secret/password }}
"#;

        let result = processor.process_template(template).await.unwrap();

        // Only the template block should be replaced
        assert!(result.contains("pass://docs/wiki/setup")); // Comment preserved  
        assert!(result.contains("pass://real/secret/password (not templated)")); // Comment preserved
        assert!(result.contains("actual_password: should_replace")); // Template replaced
    }

    #[test]
    fn static_secret_resolver_builder_pattern() {
        let resolver = StaticSecretResolver::new()
            .with_secret("share1", "item1", "field1", "value1")
            .with_secret("share2", "item2", "field2", "value2");

        // Test that secrets are stored correctly
        assert_eq!(resolver.secrets.len(), 2);
        assert_eq!(
            resolver.secrets.get("share1:item1:field1"),
            Some(&"value1".to_string())
        );
        assert_eq!(
            resolver.secrets.get("share2:item2:field2"),
            Some(&"value2".to_string())
        );
    }

    #[test]
    fn handlebars_template_validation() {
        use handlebars::Handlebars;

        let handlebars = Handlebars::new();

        // Valid template syntax (though will fail to render due to unregistered helpers)
        let valid_syntax_templates = vec![
            "plain text with no templates",
            "{{ some_variable }}", // Simple variable
            "text with {{ variable }} embedded",
        ];

        for template in valid_syntax_templates {
            let result = handlebars.render_template(
                template,
                &json!({"some_variable": "value", "variable": "test"}),
            );
            assert!(
                result.is_ok(),
                "Template syntax should be valid: {template}"
            );
        }

        // Invalid templates (malformed handlebars syntax)
        let invalid_templates = vec![
            "{{ unclosed template", // Missing closing }}
            "{{ {{ nested }} }}",   // Nested braces
        ];

        for template in invalid_templates {
            let result = handlebars.render_template(template, &json!({}));
            assert!(result.is_err(), "Template should be invalid: {template}");
        }
    }
}
