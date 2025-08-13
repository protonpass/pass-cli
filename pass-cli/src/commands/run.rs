use anyhow::{Context, Result, anyhow, bail};
use pass::PassClient;
use regex::Regex;
use std::collections::HashMap;
use std::env;
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Command, Stdio};
use std::thread;
use std::thread::JoinHandle;

use super::secret_resolver::{PassClientResolver, SecretCache, SecretReference, find_pass_uris};

const STDIN_BUFF_SIZE: usize = 1024;

#[derive(Debug)]
struct EnvVar {
    name: String,
    value: String,
}

#[derive(Debug)]
struct DotenvFile {
    vars: Vec<EnvVar>,
}

fn load_dotenv_file(path: &str) -> Result<DotenvFile> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read env file: {path}"))?;

    let mut vars = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Parse KEY=VALUE format
        if let Some(eq_pos) = line.find('=') {
            let name = line[..eq_pos].trim().to_string();
            let value = line[eq_pos + 1..].trim().to_string();

            // Remove quotes if present
            let value = if (value.starts_with('"') && value.ends_with('"'))
                || (value.starts_with('\'') && value.ends_with('\''))
            {
                value[1..value.len() - 1].to_string()
            } else {
                value
            };

            vars.push(EnvVar { name, value });
        } else {
            eprintln!(
                "Warning: Invalid line {} in {}: {}",
                line_num + 1,
                path,
                line
            );
        }
    }

    Ok(DotenvFile { vars })
}

fn get_environment_variables(env_files: &[String]) -> Result<Vec<EnvVar>> {
    let mut all_vars = Vec::new();

    // Load environment variables from the current process
    for (name, value) in env::vars() {
        all_vars.push(EnvVar { name, value });
    }

    // Load variables from .env files (these take precedence)
    for env_file in env_files {
        let dotenv_file = load_dotenv_file(env_file)?;

        // Remove any existing variables with the same name from process env
        all_vars.retain(|var| {
            !dotenv_file
                .vars
                .iter()
                .any(|dotenv_var| dotenv_var.name == var.name)
        });

        // Add variables from the .env file
        all_vars.extend(dotenv_file.vars);
    }

    Ok(all_vars)
}

fn find_secret_references(env_vars: &[EnvVar]) -> Result<HashMap<String, Vec<String>>> {
    let mut secret_map: HashMap<String, Vec<String>> = HashMap::new();

    for env_var in env_vars {
        let uris = find_pass_uris(&env_var.value)?;
        if !uris.is_empty() {
            secret_map.insert(env_var.name.clone(), uris);
        }
    }

    Ok(secret_map)
}

async fn resolve_secrets_and_create_env(
    env_vars: Vec<EnvVar>,
    secret_refs: HashMap<String, Vec<String>>,
    client: PassClient,
) -> Result<HashMap<String, String>> {
    let resolver = PassClientResolver::new(client);
    let cache = SecretCache::new();
    let mut resolved_env: HashMap<String, String> = HashMap::new();

    for env_var in env_vars {
        if let Some(uris) = secret_refs.get(&env_var.name) {
            // Resolve secrets in this variable
            let mut resolved_value = env_var.value.clone();

            for uri in uris {
                let secret_ref = SecretReference::parse(uri).with_context(|| {
                    format!("Invalid secret reference in {}: {}", env_var.name, uri)
                })?;

                let secret_value = cache
                    .get_or_resolve(&secret_ref, &resolver)
                    .await
                    .with_context(|| {
                        format!(
                            "Failed to resolve secret {} in variable {}",
                            uri, env_var.name
                        )
                    })?;

                resolved_value = resolved_value.replace(uri, &secret_value);
            }

            resolved_env.insert(env_var.name, resolved_value);
        }
    }

    Ok(resolved_env)
}

/// Create a regex pattern to match secrets for output masking
fn create_masking_regex(resolved_env: &HashMap<String, String>) -> Result<Option<Regex>> {
    let mut secret_values = Vec::new();

    for (name, value) in resolved_env {
        // Only collect values that originally contained pass:// URIs
        let original_env_value = env::var(name).unwrap_or_default();
        if !find_pass_uris(&original_env_value)
            .unwrap_or_default()
            .is_empty()
        {
            // Escape special regex characters and add to list
            let escaped = regex::escape(value);
            if !escaped.is_empty() && escaped.len() > 3 {
                // Only mask meaningful secrets
                secret_values.push(escaped);
            }
        }
    }

    if secret_values.is_empty() {
        return Ok(None);
    }

    let pattern = format!(r"({})", secret_values.join("|"));
    let regex =
        Regex::new(&pattern).map_err(|e| anyhow!("Failed to create masking regex: {}", e))?;

    Ok(Some(regex))
}

fn mask_line(line: &str, masking_regex: &Option<Regex>) -> String {
    if let Some(regex) = masking_regex {
        regex
            .replace_all(line, "<concealed by Proton Pass>")
            .to_string()
    } else {
        line.to_string()
    }
}

#[cfg(not(target_os = "windows"))]
async fn kill_process_by_pid(pid: i32) {
    // On Unix systems, send SIGTERM first, then SIGKILL if needed
    unsafe {
        const SIGKILL_GRACE_TIME_MS: u64 = 2_000;

        libc::kill(pid, libc::SIGTERM);

        // Give the process a moment to terminate gracefully
        tokio::time::sleep(tokio::time::Duration::from_millis(SIGKILL_GRACE_TIME_MS)).await;

        // Check if process is still running by sending signal 0
        if libc::kill(pid, 0) == 0 {
            // Process is still running, force kill
            libc::kill(pid, libc::SIGKILL);
        }
    }
}

#[cfg(target_os = "windows")]
async fn kill_process_by_pid(pid: i32) {
    // On non-Unix systems (Windows), we'll use taskkill command as a simple approach
    let _ = std::process::Command::new("taskkill")
        .args(&["/PID", &pid.to_string(), "/F"])
        .output();
}

fn handle_stream<R: Read + Send + 'static>(
    stream: R,
    masking_regex: Option<Regex>,
    is_stderr: bool,
) -> JoinHandle<()> {
    let reader = BufReader::new(stream);
    let masking_regex_stdout = masking_regex.clone();
    thread::spawn(move || {
        for line in reader.lines() {
            match line {
                Ok(line) => {
                    let masked_line = mask_line(&line, &masking_regex_stdout);
                    println!("{masked_line}");
                }
                Err(e) => {
                    if is_stderr {
                        eprintln!("Error reading stderr: {e}")
                    } else {
                        eprintln!("Error reading stdout: {e}")
                    }
                }
            }
        }
    })
}

fn handle_stdin<W: Write + Send + 'static>(mut child_stdin: W) -> JoinHandle<()> {
    thread::spawn(move || {
        let mut stdin = std::io::stdin();
        let mut buff = [0; STDIN_BUFF_SIZE];
        loop {
            match stdin.read(&mut buff) {
                Ok(0) => break, // EOF
                Ok(l) => {
                    let content = &buff[0..l];
                    if let Err(e) = child_stdin.write_all(content) {
                        eprintln!("Error writing to child stdin: {e}");
                        break;
                    }
                    if let Err(e) = child_stdin.flush() {
                        eprintln!("Error flushing child stdin: {e}");
                        break;
                    }
                }
                Err(e) => {
                    eprintln!("Error reading from stdin: {e}");
                    break;
                }
            }
        }
    })
}

async fn execute_command(
    command_args: &[String],
    resolved_env: HashMap<String, String>,
    no_masking: bool,
) -> Result<i32> {
    if command_args.is_empty() {
        bail!("No command provided");
    }

    let program = &command_args[0];
    let args = &command_args[1..];

    // Create masking regex if needed
    let masking_regex = if no_masking {
        None
    } else {
        create_masking_regex(&resolved_env)?
    };

    // Start the subprocess
    let mut child = Command::new(program)
        .args(args)
        .envs(&resolved_env)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("Failed to execute command: {program}"))?;

    // Store child process ID for signal handling
    let child_pid = child.id();

    // Get stdin, stdout and stderr handles
    let stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    let stdin_handle = handle_stdin(stdin);
    let stdout_handle = handle_stream(stdout, masking_regex.clone(), false);
    let stderr_handle = handle_stream(stderr, masking_regex, true);

    // Use tokio::select! to handle both child completion and Ctrl+C
    let exit_status = tokio::select! {
        // Wait for child process to complete
        result = tokio::task::spawn_blocking(move || child.wait()) => {
            result.map_err(|e| anyhow!("Task join error: {}", e))?
                .context("Failed to wait for child process")?
        },
        // Handle Ctrl+C signal
        _ = tokio::signal::ctrl_c() => {
            eprintln!("\nReceived Ctrl+C, terminating child process...");

            // Kill the child process
            kill_process_by_pid(child_pid as i32).await;

            std::process::exit(130); // Standard exit code for Ctrl+C (128 + SIGINT)
        }
    };

    // Wait for I/O handling threads to complete
    stdout_handle
        .join()
        .map_err(|_| anyhow!("Failed to join stdout thread"))?;
    stderr_handle
        .join()
        .map_err(|_| anyhow!("Failed to join stderr thread"))?;
    stdin_handle
        .join()
        .map_err(|_| anyhow!("Failed to join stdin thread"))?;

    Ok(exit_status.code().unwrap_or(-1))
}

pub async fn run(
    env_files: Vec<String>,
    no_masking: bool,
    command_args: Vec<String>,
    client: PassClient,
) -> Result<()> {
    // Get all environment variables (process + .env files)
    let env_vars =
        get_environment_variables(&env_files).context("Failed to load environment variables")?;

    // Check if any environment variables contain secret references
    let secret_refs =
        find_secret_references(&env_vars).context("Failed to find secret references")?;

    if secret_refs.is_empty() {
        // No secrets found, execute command directly with current environment
        if command_args.is_empty() {
            bail!("No command provided");
        }

        let program = &command_args[0];
        let args = &command_args[1..];

        let exit_status = Command::new(program)
            .args(args)
            .status()
            .with_context(|| format!("Failed to execute command: {program}"))?;

        std::process::exit(exit_status.code().unwrap_or(-1));
    }

    // Resolve secrets and create environment
    let resolved_env = resolve_secrets_and_create_env(env_vars, secret_refs, client)
        .await
        .context("Failed to resolve secrets")?;

    // Execute the command with resolved environment
    let exit_code = execute_command(&command_args, resolved_env, no_masking)
        .await
        .context("Failed to execute command with secrets")?;

    std::process::exit(exit_code);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_dotenv_file() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        // Create a temporary .env file
        let content = r#"
# This is a comment
DB_PASSWORD=pass://prod/db/password
API_KEY="pass://api/service/key"
REGULAR_VAR=normal_value

# Another comment
QUOTED_VAR='some value'
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(content.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let result = load_dotenv_file(temp_file.path().to_str().unwrap()).unwrap();

        assert_eq!(result.vars.len(), 4);
        assert_eq!(result.vars[0].name, "DB_PASSWORD");
        assert_eq!(result.vars[0].value, "pass://prod/db/password");
        assert_eq!(result.vars[1].name, "API_KEY");
        assert_eq!(result.vars[1].value, "pass://api/service/key"); // Quotes removed
        assert_eq!(result.vars[2].name, "REGULAR_VAR");
        assert_eq!(result.vars[2].value, "normal_value");
        assert_eq!(result.vars[3].name, "QUOTED_VAR");
        assert_eq!(result.vars[3].value, "some value"); // Quotes removed
    }

    #[test]
    fn test_find_secret_references() {
        let env_vars = vec![
            EnvVar {
                name: "DB_PASSWORD".to_string(),
                value: "pass://prod/db/password".to_string(),
            },
            EnvVar {
                name: "API_KEY".to_string(),
                value: "pass://api/service/key".to_string(),
            },
            EnvVar {
                name: "NORMAL_VAR".to_string(),
                value: "normal_value".to_string(),
            },
            EnvVar {
                name: "MIXED_VAR".to_string(),
                value: "prefix_pass://mixed/item/field_suffix".to_string(),
            },
        ];

        let secret_refs = find_secret_references(&env_vars).unwrap();

        assert_eq!(secret_refs.len(), 3);
        assert!(secret_refs.contains_key("DB_PASSWORD"));
        assert!(secret_refs.contains_key("API_KEY"));
        assert!(secret_refs.contains_key("MIXED_VAR"));
        assert!(!secret_refs.contains_key("NORMAL_VAR"));

        assert_eq!(secret_refs["DB_PASSWORD"], vec!["pass://prod/db/password"]);
        assert_eq!(secret_refs["API_KEY"], vec!["pass://api/service/key"]);
        assert_eq!(
            secret_refs["MIXED_VAR"],
            vec!["pass://mixed/item/field_suffix"]
        );
    }

    #[test]
    fn test_mask_line() {
        let regex = Regex::new(r"(secret123|password456)").ok();

        let line1 = "The password is secret123 and key is password456";
        let masked1 = mask_line(line1, &regex);
        assert_eq!(
            masked1,
            "The password is <concealed by Proton Pass> and key is <concealed by Proton Pass>"
        );

        let line2 = "No secrets here";
        let masked2 = mask_line(line2, &regex);
        assert_eq!(masked2, "No secrets here");

        let line3 = "Multiple secret123 occurrences of secret123";
        let masked3 = mask_line(line3, &regex);
        assert_eq!(
            masked3,
            "Multiple <concealed by Proton Pass> occurrences of <concealed by Proton Pass>"
        );
    }
}
