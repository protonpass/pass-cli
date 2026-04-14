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
use std::path::Path;

const TRACK_FILE_NAME: &str = "update_track";

pub async fn get_persistent_track(base_dir: &Path) -> Result<Option<String>> {
    let track_path = base_dir.join(TRACK_FILE_NAME);

    if tokio::fs::metadata(&track_path).await.is_err() {
        return Ok(None);
    }

    let contents = tokio::fs::read_to_string(&track_path)
        .await
        .context("Failed to read track file")?;

    let track = contents.trim().to_string();

    if track.is_empty() {
        Ok(None)
    } else {
        Ok(Some(track))
    }
}

pub async fn set_persistent_track(base_dir: &Path, track: &str) -> Result<()> {
    let track_path = base_dir.join(TRACK_FILE_NAME);
    let track = track.trim();

    tokio::fs::write(&track_path, track)
        .await
        .context("Failed to write track file")?;

    Ok(())
}

// Get the release track to be used (from persistent storage, env var, or default)
// Priority: persistent storage > environment variable > default
pub async fn get_release_track(base_dir: &Path) -> Result<String> {
    if let Some(track) = get_persistent_track(base_dir).await? {
        return Ok(track);
    }

    // Fall back to environment variable
    #[cfg(not(debug_assertions))]
    {
        const ENV_RELEASE_CHANNEL: &str = "PROTON_PASS_RELEASE_CHANNEL";
        if let Ok(channel) = std::env::var(ENV_RELEASE_CHANNEL) {
            let channel = channel.trim().to_string();
            if !channel.is_empty() {
                return Ok(channel);
            }
        }
    }

    // Default to stable
    Ok("stable".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_set_and_get_track() {
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path();

        // Initially no track
        let track = get_persistent_track(base_dir).await.unwrap();
        assert_eq!(track, None);

        // Set track
        let track_str = "test-track";
        set_persistent_track(base_dir, track_str).await.unwrap();

        // Get track
        let track = get_persistent_track(base_dir).await.unwrap();
        assert_eq!(track, Some(track_str.to_string()));
    }

    #[tokio::test]
    async fn test_get_release_track_defaults_to_stable() {
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path();

        let track = get_release_track(base_dir).await.unwrap();
        assert_eq!(track, "stable");
    }

    #[tokio::test]
    async fn test_get_release_track_uses_persistent() {
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path();

        let track_str = "testingtrack";
        set_persistent_track(base_dir, track_str).await.unwrap();

        let track = get_release_track(base_dir).await.unwrap();
        assert_eq!(track, track_str);
    }
}
