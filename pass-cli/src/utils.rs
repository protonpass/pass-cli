use anyhow::Context;
use std::path::PathBuf;

const PROTON_PASS_SESSION_DIR_ENV: &str = "PROTON_PASS_SESSION_DIR";

pub fn ask_for_input(prompt: &str, secure: bool) -> anyhow::Result<String> {
    if secure {
        let input = rpassword::prompt_password(prompt).context("Error prompting for password")?;
        Ok(input.replace("\n", "").trim().to_string())
    } else {
        let stdin = std::io::stdin();
        loop {
            let mut username = String::new();
            println!("{prompt}");

            stdin.read_line(&mut username)?;

            if !username.trim().is_empty() {
                return Ok(username.replace("\n", "").trim().to_string());
            } else {
                eprintln!("Username is empty");
            }
        }
    }
}

pub fn get_base_dir() -> anyhow::Result<PathBuf> {
    // Check for environment variable override first
    let proton_dir = if let Ok(custom_dir) = std::env::var(PROTON_PASS_SESSION_DIR_ENV) {
        PathBuf::from(custom_dir)
    } else {
        // Use platform-specific data directory
        let data_dir = dirs::data_dir()
            .context("Failed to determine data directory for this platform")?;
        data_dir.join("proton-pass-cli")
    };

    // Create a .session subfolder (just like before, but in the platform-specific location)
    let session_dir = proton_dir.join(".session");
    
    // Create the directory if it doesn't exist
    std::fs::create_dir_all(&session_dir).context("Error creating session directory")?;

    // Return the canonicalized (absolute) path
    let session_dir_absolute =
        std::fs::canonicalize(&session_dir).context("Error getting absolute path")?;
    Ok(session_dir_absolute)
}
