use anyhow::Context;
use std::path::PathBuf;

const ENV_BASE_DIR: &str = "BASE_DIR";
const PASS_SESSION_DIR: &str = "pass-cli-session";

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
                warn!("Username is empty");
            }
        }
    }
}

pub fn get_base_dir() -> anyhow::Result<PathBuf> {
    let base_dir = match std::env::var(ENV_BASE_DIR) {
        Ok(base_dir) => PathBuf::from(base_dir),
        Err(_) => {
            let current_dir = std::env::current_dir().context("Error getting current dir")?;
            let session_path = current_dir.join(".session");
            std::fs::create_dir_all(&session_path).context("Error creating base dir")?;
            session_path
        }
    }
    .join(PASS_SESSION_DIR)
    .to_path_buf();
    std::fs::create_dir_all(&base_dir).context("Error creating base session dir")?;

    let base_dir_absolute =
        std::fs::canonicalize(&base_dir).context("error getting absolute path")?;
    Ok(base_dir_absolute)
}
