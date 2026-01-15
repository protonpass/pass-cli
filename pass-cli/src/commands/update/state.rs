use anyhow::{Context, Result};
use std::path::Path;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const LAST_CHECK_FILE: &str = ".last_update_check";
const UPDATE_DAYS_CHECK_INTERVAL: i64 = 3;

pub async fn get_last_check(base_dir: &Path) -> Result<Option<jiff::Zoned>> {
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

    let trimmed = contents.trim();

    // Try to parse as i64 (new format)
    if let Ok(timestamp) = trimmed.parse::<i64>()
        && let Ok(ts) = jiff::Timestamp::from_second(timestamp)
    {
        return Ok(Some(ts.to_zoned(jiff::tz::TimeZone::UTC)));
    }

    // Try to parse as Zoned datetime (old chrono format for backwards compatibility)
    if let Ok(zoned) = trimmed.parse::<jiff::Zoned>() {
        return Ok(Some(zoned));
    }

    // If both fail, return None to force re-check (file will be updated with new format)
    Ok(None)
}

pub async fn update_last_check(base_dir: &Path) -> Result<()> {
    let file_path = base_dir.join(LAST_CHECK_FILE);
    let now = jiff::Timestamp::now().as_second();
    let timestamp = now.to_string();

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
            let now = jiff::Timestamp::now().to_zoned(jiff::tz::TimeZone::UTC);
            let duration = now.timestamp().as_second() - last.timestamp().as_second();
            let days = duration / (24 * 60 * 60);
            Ok(days >= UPDATE_DAYS_CHECK_INTERVAL)
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
