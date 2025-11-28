use anyhow::Result;

#[allow(unused_imports)]
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallSource {
    Standard,
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    Homebrew,
    #[cfg_attr(not(windows), allow(dead_code))]
    Chocolatey,
}

impl InstallSource {
    pub fn print_instructions(&self) {
        match self {
            InstallSource::Standard => {
                // No special instructions needed - standard update flow
            }
            InstallSource::Homebrew => {
                eprintln!("\nIt looks like pass-cli was installed via Homebrew.");
                eprintln!("To update, please run:");
                eprintln!("    brew update");
                eprintln!("    brew upgrade pass-cli");
            }
            InstallSource::Chocolatey => {
                eprintln!("\nIt looks like pass-cli was installed via Chocolatey.");
                eprintln!("To update, please run:");
                eprintln!("    choco upgrade pass-cli");
            }
        }
    }
}

pub fn get_install_source() -> Result<InstallSource> {
    // Check for Homebrew installation (macOS)
    #[cfg(target_os = "macos")]
    {
        use anyhow::Context;
        let current_exe =
            std::env::current_exe().context("Failed to get current executable path")?;
        let resolved_exe = std::fs::canonicalize(&current_exe)
            .context("Failed to resolve current executable path")?;
        if is_homebrew_install(resolved_exe.as_path()) {
            return Ok(InstallSource::Homebrew);
        }
    }

    // Check for Chocolatey installation (Windows)
    #[cfg(windows)]
    {
        use anyhow::Context;
        let current_exe =
            std::env::current_exe().context("Failed to get current executable path")?;
        let resolved_exe = std::fs::canonicalize(&current_exe)
            .context("Failed to resolve current executable path")?;

        let exe_path_str = resolved_exe.to_string_lossy();
        if is_chocolatey_install(&exe_path_str) {
            return Ok(InstallSource::Chocolatey);
        }
    }

    // Default to standard installation
    Ok(InstallSource::Standard)
}

#[cfg(target_os = "macos")]
fn is_homebrew_install(exe_path: &Path) -> bool {
    let exe_path_str = exe_path.to_string_lossy();

    // Check common Homebrew paths
    // macOS Intel: /usr/local/Cellar or /usr/local/bin
    // macOS ARM: /opt/homebrew/Cellar or /opt/homebrew/bin
    if exe_path_str.contains("/Cellar/pass-cli/")
        || exe_path_str.contains("/opt/homebrew/")
        || exe_path_str.contains("/usr/local/Cellar/")
    {
        debug!(
            "Detected homebrew installation due to the exe path containing a known homebrew path"
        );
        return true;
    }

    // Check if the binary is a symlink from homebrew's bin directory
    // to a Cellar location (this covers /usr/local/bin/pass-cli -> /usr/local/Cellar/...)
    if let Some(parent) = exe_path.parent() {
        let parent_str = parent.to_string_lossy();
        if parent_str.contains("/homebrew/") || parent_str.ends_with("/usr/local/bin") {
            // Additional verification: check if Homebrew directory structure exists
            if let Some(homebrew_prefix) = get_homebrew_prefix(&parent_str) {
                let cellar_path = format!("{}/Cellar/pass-cli", homebrew_prefix);
                if Path::new(&cellar_path).exists() {
                    debug!("Detected homebrew installation using fallback strategy");
                    return true;
                }
            }
        }
    }

    false
}

#[cfg(target_os = "macos")]
fn get_homebrew_prefix(path: &str) -> Option<String> {
    if path.contains("/opt/homebrew") {
        Some("/opt/homebrew".to_string())
    } else if path.contains("/usr/local") {
        Some("/usr/local".to_string())
    } else {
        None
    }
}

#[cfg(windows)]
fn is_chocolatey_install(exe_path_str: &str) -> bool {
    // Chocolatey typically installs to C:\ProgramData\chocolatey or C:\tools
    exe_path_str.contains("\\chocolatey\\")
        || exe_path_str.contains("\\ProgramData\\chocolatey\\")
        || (exe_path_str.contains("\\tools\\") && is_chocolatey_managed(exe_path_str))
}

#[cfg(windows)]
fn is_chocolatey_managed(exe_path_str: &str) -> bool {
    // Check if .chocolateyInstall.ps1 or similar files exist in parent directory
    if let Some(parent_path) = Path::new(exe_path_str).parent() {
        let choco_marker = parent_path.join(".chocolateyInstall.ps1");
        if choco_marker.exists() {
            return true;
        }
    }
    false
}
