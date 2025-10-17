use anyhow::{Context, Result, anyhow};
use std::path::{Path, PathBuf};

pub async fn replace_binary(new_binary: &Path) -> Result<()> {
    let current_exe = std::env::current_exe().context("Failed to get current executable path")?;

    // Resolve symlinks to get the actual binary
    let current_exe = tokio::fs::canonicalize(&current_exe)
        .await
        .context("Failed to resolve current executable path")?;

    #[cfg(unix)]
    {
        replace_binary_unix(&current_exe, new_binary).await
    }

    #[cfg(windows)]
    {
        replace_binary_windows(&current_exe, new_binary).await
    }
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
async fn replace_binary_windows(current_exe: &Path, new_binary: &Path) -> Result<()> {
    use std::os::windows::process::CommandExt;
    use tokio::process::Command;

    // On Windows, the running executable is locked. We need to use a helper script.
    // We'll use self-replace technique: spawn a detached process that waits for us to exit,
    // then replaces the binary.

    // Create a batch script that will perform the replacement
    let temp_dir = std::env::temp_dir();
    let script_path = temp_dir.join(format!("pass-cli-update-{}.bat", std::process::id()));

    let script_content = format!(
        r#"@echo off
:wait
timeout /t 1 /nobreak >nul
tasklist /FI "PID eq {}" 2>nul | find "{}" >nul
if not errorlevel 1 goto wait
move /Y "{}" "{}"
del "%~f0"
"#,
        std::process::id(),
        std::process::id(),
        new_binary.display(),
        current_exe.display()
    );

    tokio::fs::write(&script_path, script_content)
        .await
        .context("Failed to create update script")?;

    // Spawn detached process to run the script
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    const DETACHED_PROCESS: u32 = 0x00000008;

    Command::new("cmd")
        .args(&["/C", "start", "/B", script_path.to_str().unwrap()])
        .creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS)
        .spawn()
        .context("Failed to spawn update helper")?;

    println!("Update will complete after this process exits.");

    Ok(())
}

fn get_backup_path(current_exe: &Path) -> PathBuf {
    let mut backup = current_exe.to_path_buf();
    let pid = std::process::id();
    let file_name = current_exe
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("protonpass");

    backup.set_file_name(format!("{file_name}.old.{pid}"));
    backup
}
