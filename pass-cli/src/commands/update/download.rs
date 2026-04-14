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

use anyhow::{Context, Result};
use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;

async fn download_and_verify(url: &str, expected_hash: &str, extension: &str) -> Result<PathBuf> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .context("Failed to create HTTP client")?;

    let response = client
        .get(url)
        .send()
        .await
        .context("Failed to download file")?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Failed to download file: HTTP {}",
            response.status()
        ));
    }

    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join(format!("pass-cli-{}{}", uuid::Uuid::new_v4(), extension));

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

#[cfg(unix)]
pub async fn download_binary(url: &str, expected_hash: &str) -> Result<PathBuf> {
    let temp_file = download_and_verify(url, expected_hash, ".download").await?;

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

    Ok(temp_file)
}

#[cfg(windows)]
fn extract_zip(zip_path: &PathBuf) -> Result<PathBuf> {
    let temp_dir = std::env::temp_dir();
    let extract_dir = temp_dir.join(format!("pass-cli-update-{}", uuid::Uuid::new_v4()));

    std::fs::create_dir_all(&extract_dir).context("Failed to create extraction directory")?;

    let file = std::fs::File::open(zip_path).context("Failed to open zip file")?;
    let mut archive = zip::ZipArchive::new(file).context("Failed to read zip archive")?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).context("Failed to read zip entry")?;
        let outpath = extract_dir.join(file.name());

        if file.is_dir() {
            std::fs::create_dir_all(&outpath).context("Failed to create directory from zip")?;
        } else {
            if let Some(p) = outpath.parent() {
                std::fs::create_dir_all(p).context("Failed to create parent directory")?;
            }
            let mut outfile =
                std::fs::File::create(&outpath).context("Failed to create extracted file")?;
            std::io::copy(&mut file, &mut outfile).context("Failed to extract file")?;
        }
    }

    Ok(extract_dir)
}

#[cfg(windows)]
pub async fn download_and_extract_zip(url: &str, expected_hash: &str) -> Result<PathBuf> {
    // Download and verify the zip file
    let temp_zip = download_and_verify(url, expected_hash, ".zip").await?;

    // Extract it
    let extract_dir = extract_zip(&temp_zip)?;

    // Clean up the zip file
    let _ = tokio::fs::remove_file(&temp_zip).await;

    Ok(extract_dir)
}
