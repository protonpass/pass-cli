use anyhow::{Context, Result};
use std::path::Path;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const LAST_CHECK_FILE: &str = ".last_update_check";

pub async fn get_last_check(base_dir: &Path) -> Result<Option<chrono::DateTime<chrono::Utc>>> {
    let file_path = base_dir.join(LAST_CHECK_FILE);

    if !file_path.exists() {
        return Ok(None);
    }

    let mut file = tokio::fs::File::open(&file_path)
        .await
        .context("Failed to open last check file")?;

    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .await
        .context("Failed to read last check file")?;

    let timestamp = chrono::DateTime::parse_from_rfc3339(contents.trim())
        .context("Failed to parse timestamp")?;

    Ok(Some(timestamp.with_timezone(&chrono::Utc)))
}

pub async fn update_last_check(base_dir: &Path) -> Result<()> {
    let file_path = base_dir.join(LAST_CHECK_FILE);
    let now = chrono::Utc::now();
    let timestamp = now.to_rfc3339();

    let mut file = tokio::fs::File::create(&file_path)
        .await
        .context("Failed to create last check file")?;

    file.write_all(timestamp.as_bytes())
        .await
        .context("Failed to write last check file")?;

    file.sync_all()
        .await
        .context("Failed to sync last check file")?;

    Ok(())
}

pub async fn should_check_for_updates(base_dir: &Path) -> Result<bool> {
    let last_check = get_last_check(base_dir).await?;

    match last_check {
        None => Ok(true), // Never checked before
        Some(last) => {
            let now = chrono::Utc::now();
            let duration = now.signed_duration_since(last);
            Ok(duration.num_days() >= 7)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_state_management() {
        let temp_dir = tempdir().unwrap();
        let base_dir = temp_dir.path();

        // Initially should have no last check
        let last_check = get_last_check(base_dir).await.unwrap();
        assert!(last_check.is_none());

        // Should check for updates
        let should_check = should_check_for_updates(base_dir).await.unwrap();
        assert!(should_check);

        // Update last check
        update_last_check(base_dir).await.unwrap();

        // Now should have a last check
        let last_check = get_last_check(base_dir).await.unwrap();
        assert!(last_check.is_some());

        // Should not check for updates (within 7 days)
        let should_check = should_check_for_updates(base_dir).await.unwrap();
        assert!(!should_check);
    }
}
