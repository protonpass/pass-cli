use anyhow::{Context, Result};
use std::collections::HashMap;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub format_version: u32,
    pub pass_cli_versions: VersionInfo,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct VersionInfo {
    pub version: String,
    pub urls: HashMap<String, HashMap<String, BinaryInfo>>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct BinaryInfo {
    pub url: String,
    pub hash: String,
}

// Certificate pinning for release builds
#[cfg(not(debug_assertions))]
fn get_pinned_certificates() -> Result<Vec<reqwest::Certificate>> {
    // TBD: add pinned certificates
    Ok(vec![])
}

#[cfg(debug_assertions)]
fn get_pinned_certificates() -> Result<Vec<reqwest::Certificate>> {
    Ok(vec![])
}

fn build_client() -> Result<reqwest::Client> {
    let mut builder = reqwest::Client::builder().timeout(std::time::Duration::from_secs(30));

    let pinned_certs = get_pinned_certificates()?;

    // In release builds with cert pinning, we can optionally disable the default
    // root certificates to ensure ONLY our pinned certs are used
    if !cfg!(debug_assertions) && !pinned_certs.is_empty() {
        builder = builder.tls_built_in_root_certs(false);
    }

    for cert in pinned_certs {
        builder = builder.add_root_certificate(cert);
    }

    builder.build().context("Failed to create HTTP client")
}

pub async fn fetch_manifest(url: &str) -> Result<Manifest> {
    let client = build_client()?;

    let response = client
        .get(url)
        .send()
        .await
        .context("Failed to fetch manifest")?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Failed to fetch manifest: HTTP {}",
            response.status()
        ));
    }

    let manifest: Manifest = response
        .json()
        .await
        .context("Failed to parse manifest JSON")?;

    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_deserialization() {
        let json = r#"{
            "formatVersion": 1,
            "passCliVersions": {
                "version": "4.5.6",
                "urls": {
                    "windows": {
                        "x86_64": {
                            "url": "https://example.com/pass-cli.exe",
                            "hash": "abcdef123456"
                        }
                    },
                    "macos": {
                        "aarch64": {
                            "url": "https://example.com/pass-cli-macos-aarch64",
                            "hash": "abcdef123456"
                        },
                        "x86_64": {
                            "url": "https://example.com/pass-cli-macos-x86_64",
                            "hash": "abcdef123456"
                        }
                    },
                    "linux": {
                        "aarch64": {
                            "url": "https://example.com/pass-cli-linux-aarch64",
                            "hash": "abcdef123456"
                        },
                        "x86_64": {
                            "url": "https://example.com/pass-cli-linux-x86_64",
                            "hash": "abcdef123456"
                        }
                    }
                }
            }
        }"#;

        let manifest: Manifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.format_version, 1);
        assert_eq!(manifest.pass_cli_versions.version, "4.5.6");

        // Verify URLs structure
        assert!(manifest.pass_cli_versions.urls.contains_key("windows"));
        assert!(manifest.pass_cli_versions.urls.contains_key("macos"));
        assert!(manifest.pass_cli_versions.urls.contains_key("linux"));

        // Verify architectures
        let macos_urls = manifest.pass_cli_versions.urls.get("macos").unwrap();
        assert!(macos_urls.contains_key("x86_64"));
        assert!(macos_urls.contains_key("aarch64"));
    }
}
