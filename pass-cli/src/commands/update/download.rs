use anyhow::{Context, Result};
use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;

pub async fn download_binary(url: &str, expected_hash: &str) -> Result<PathBuf> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .context("Failed to create HTTP client")?;

    let response = client
        .get(url)
        .send()
        .await
        .context("Failed to download binary")?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Failed to download binary: HTTP {}",
            response.status()
        ));
    }

    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join(format!("pass-cli-{}.download", uuid::Uuid::new_v4()));

    // Download and hash in chunks, with cleanup on error
    let result = async {
        let mut file = tokio::fs::File::create(&temp_file)
            .await
            .context("Failed to create temp file")?;

        let mut hasher = Sha256::new();
        let mut stream = response.bytes_stream();

        // Download in chunks (typically ~4MB), updating hash as we stream
        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.context("Failed to read chunk")?;
            hasher.update(&chunk);
            file.write_all(&chunk)
                .await
                .context("Failed to write chunk to temp file")?;
        }

        file.sync_all().await.context("Failed to sync temp file")?;

        // Verify hash after download completes
        let computed_hash = format!("{:x}", hasher.finalize());
        if computed_hash != expected_hash {
            return Err(anyhow::anyhow!(
                "Downloaded file failed verification. Expected hash: {expected_hash}, got: {computed_hash}",
            ));
        }

        // Set executable permissions on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = tokio::fs::metadata(&temp_file)
                .await
                .context("Failed to get temp file metadata")?
                .permissions();
            perms.set_mode(0o755);
            tokio::fs::set_permissions(&temp_file, perms)
                .await
                .context("Failed to set temp file permissions")?;
        }

        Ok::<(), anyhow::Error>(())
    }
    .await;

    match result {
        Ok(_) => Ok(temp_file),
        Err(e) => {
            // Clean up temp file on error
            let _ = tokio::fs::remove_file(&temp_file).await;
            Err(e)
        }
    }
}
