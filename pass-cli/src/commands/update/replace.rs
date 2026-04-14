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

#[cfg(unix)]
use anyhow::anyhow;
use anyhow::{Context, Result};
use std::path::Path;

#[cfg(unix)]
use std::path::PathBuf;

#[cfg(unix)]
pub async fn replace_binary(new_binary: &Path) -> Result<()> {
    let current_exe = std::env::current_exe().context("Failed to get current executable path")?;

    // Resolve symlinks to get the actual binary
    let current_exe = tokio::fs::canonicalize(&current_exe)
        .await
        .context("Failed to resolve current executable path")?;

    replace_binary_unix(&current_exe, new_binary).await
}

#[cfg(unix)]
async fn replace_binary_unix(current_exe: &Path, new_binary: &Path) -> Result<()> {
    // Get backup path
    let backup_path = get_backup_path(current_exe);

    // Try to rename current binary to backup path
    if let Err(e) = tokio::fs::rename(&current_exe, &backup_path).await {
        // Check if it's a permission error
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            return Err(anyhow!(
                "Cannot write to install location ({}). Re-run with appropriate permissions or reinstall to a user directory.",
                current_exe.display()
            ));
        }
        return Err(anyhow!("Failed to create backup: {e}"));
    }

    // Copy new binary to target location (our original path)
    match tokio::fs::copy(new_binary, &current_exe).await {
        Ok(_) => {
            // Success. Clean up temp file and backup
            let _ = tokio::fs::remove_file(new_binary).await;
            let _ = tokio::fs::remove_file(&backup_path).await;
            Ok(())
        }
        Err(e) => {
            // Failed to copy, try to restore backup
            let _ = tokio::fs::rename(&backup_path, &current_exe).await;
            Err(anyhow!("Failed to copy new binary: {e}"))
        }
    }
}

#[cfg(windows)]
fn strip_extended_length_path_prefix(path: &str) -> String {
    // Remove the \\?\ prefix that Windows uses for extended-length paths
    // This prefix doesn't work with batch commands like xcopy
    if path.starts_with(r"\\?\") {
        path[4..].to_string()
    } else {
        path.to_string()
    }
}

#[cfg(windows)]
pub async fn replace_binary_from_dir(source_dir: &Path) -> Result<()> {
    use tokio::process::Command;

    // On Windows, the running executable is locked. We need to use a helper script.
    // We'll use self-replace technique: spawn a detached process that waits for us to exit,
    // then replaces the binary.

    let current_exe = std::env::current_exe().context("Failed to get current executable path")?;

    // Resolve symlinks to get the actual binary
    let current_exe = tokio::fs::canonicalize(&current_exe)
        .await
        .context("Failed to resolve current executable path")?;

    // Get the installation directory
    let install_dir = current_exe
        .parent()
        .context("Failed to get installation directory")?;

    // Create a batch script that will perform the replacement
    let temp_dir = std::env::temp_dir();
    let script_path = temp_dir.join(format!("pass-cli-update-{}.bat", std::process::id()));

    // Convert paths to strings for batch script
    // Strip the Windows extended-length path prefix (\\?\) which doesn't work with batch commands
    let source_dir_str = strip_extended_length_path_prefix(&source_dir.to_string_lossy());
    let install_dir_str = strip_extended_length_path_prefix(&install_dir.to_string_lossy());

    let script_path_str = script_path.to_string_lossy().to_string();

    let script_content = format!(
        "@echo off\r\n\
        :wait\r\n\
        timeout /t 1 /nobreak >nul 2>&1\r\n\
        tasklist /FI \"PID eq {pid}\" 2>nul | find \"{pid}\" >nul 2>&1\r\n\
        if not errorlevel 1 goto wait\r\n\
        xcopy /Y /E /I /Q \"{source}\\*\" \"{dest}\\\" >nul 2>&1\r\n\
        rmdir /S /Q \"{source}\" >nul 2>&1\r\n\
        del /F /Q \"{script}\" >nul 2>&1\r\n",
        pid = std::process::id(),
        source = source_dir_str,
        dest = install_dir_str,
        script = script_path_str
    );

    tokio::fs::write(&script_path, script_content)
        .await
        .context("Failed to create update script")?;

    // Restrict the batch script to the current user so other users cannot
    // read install paths or tamper with the script before it executes.
    if let Err(e) =
        crate::platform::windows_permissions::restrict_file_to_current_user(&script_path)
    {
        warn!("Failed to restrict update script permissions: {e:#}");
    }

    // Spawn detached process to run the script without creating a visible window
    // We use 'start /B' to run in background, and CREATE_NO_WINDOW to prevent console creation
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    Command::new("cmd")
        .args(&["/C", "start", "/B", "cmd", "/C", &script_path_str])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .context("Failed to spawn update helper")?;

    println!("Update will complete after this process exits.");

    Ok(())
}

#[cfg(unix)]
fn get_backup_path(current_exe: &Path) -> PathBuf {
    let mut backup = current_exe.to_path_buf();
    let pid = std::process::id();
    let file_name = current_exe
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("pass-cli");

    backup.set_file_name(format!("{file_name}.old.{pid}"));
    backup
}
