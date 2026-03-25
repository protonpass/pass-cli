use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Stdio;

pub fn get_default_pid_file() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("Could not determine home directory"))?;
    Ok(home.join(".ssh").join("proton-pass-agent.pid"))
}

#[derive(Serialize, Deserialize)]
struct DaemonInfo {
    pid: u32,
    socket_path: PathBuf,
    log_file: Option<PathBuf>,
}

fn write_pid_file(path: &Path, info: &DaemonInfo) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).context("Failed to create PID file directory")?;
    }
    let content = serde_json::to_string_pretty(info).context("Failed to serialize PID file")?;
    std::fs::write(path, content).context("Failed to write PID file")
}

fn read_pid_file(path: &Path) -> Result<DaemonInfo> {
    let content = std::fs::read_to_string(path).context("Failed to read PID file")?;
    serde_json::from_str(&content).context("Failed to parse PID file")
}

pub fn run_daemon_start(
    socket_path: Option<String>,
    share_id: Option<String>,
    vault_name: Option<String>,
    refresh_interval: u64,
    create_new_identities: Option<String>,
    pid_file: Option<PathBuf>,
    log_file: Option<PathBuf>,
) -> Result<()> {
    let pid_file = pid_file.map(Ok).unwrap_or_else(get_default_pid_file)?;
    let socket_path_buf = socket_path
        .as_deref()
        .map(PathBuf::from)
        .map(Ok)
        .unwrap_or_else(super::get_default_socket_path)?;

    if pid_file.exists() {
        if let Ok(info) = read_pid_file(&pid_file)
            && is_process_running(info.pid)
        {
            return Err(anyhow!(
                "Daemon is already running (PID: {}).\nRun 'ssh-agent daemon stop' to stop it first.",
                info.pid
            ));
        }
        let _ = std::fs::remove_file(&pid_file);
    }

    let exe = std::env::current_exe().context("Failed to determine current executable path")?;

    // Reconstruct the `ssh-agent start` argument list from the parsed options.
    // We always pass --socket-path explicitly so the daemon uses the same socket
    // path that we will record in the PID file (and show to the user).
    let mut args = vec!["ssh-agent".to_string(), "start".to_string()];
    args.push("--socket-path".into());
    args.push(socket_path_buf.to_string_lossy().into_owned());
    if let Some(ref si) = share_id {
        args.push("--share-id".into());
        args.push(si.clone());
    }
    if let Some(ref vn) = vault_name {
        args.push("--vault-name".into());
        args.push(vn.clone());
    }
    args.push("--refresh-interval".into());
    args.push(refresh_interval.to_string());
    if let Some(ref cni) = create_new_identities {
        args.push("--create-new-identities".into());
        args.push(cni.clone());
    }

    let mut cmd = std::process::Command::new(&exe);
    cmd.args(&args).stdin(Stdio::null());

    if let Some(ref log_path) = log_file {
        if let Some(parent) = log_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create log directory: {}", parent.display()))?;
        }
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)
            .with_context(|| format!("Failed to open log file: {}", log_path.display()))?;
        let file2 = file
            .try_clone()
            .context("Failed to clone log file handle")?;
        cmd.stdout(file).stderr(file2);
    } else {
        cmd.stdout(Stdio::null()).stderr(Stdio::null());
    }

    platform_setup_daemon(&mut cmd);

    let child = cmd.spawn().context("Failed to spawn daemon process")?;
    let pid = child.id();

    // Resolve the log file to an absolute path so it is always displayed and
    // stored in a way that makes sense regardless of the user's working directory.
    let abs_log_file = log_file
        .as_deref()
        .map(std::fs::canonicalize)
        .transpose()
        .context("Failed to resolve log file path")?;

    write_pid_file(
        &pid_file,
        &DaemonInfo {
            pid,
            socket_path: socket_path_buf.clone(),
            log_file: abs_log_file.clone(),
        },
    )?;

    println!("Daemon started (PID: {})", pid);
    println!("PID file: {}", pid_file.display());
    if let Some(ref log) = abs_log_file {
        println!("Log file: {}", log.display());
    } else {
        println!("Logs: discarded (use --log-file to capture them)");
    }
    println!();
    println!("To connect to the agent, set SSH_AUTH_SOCK:");
    #[cfg(unix)]
    println!("  export SSH_AUTH_SOCK={}", socket_path_buf.display());
    #[cfg(windows)]
    println!("  $env:SSH_AUTH_SOCK={}", socket_path_buf.display());

    Ok(())
}

pub fn run_daemon_status(pid_file: Option<PathBuf>) -> Result<()> {
    let pid_file = pid_file.map(Ok).unwrap_or_else(get_default_pid_file)?;

    if !pid_file.exists() {
        println!("Status:   stopped");
        println!("PID file: {} (not found)", pid_file.display());
        return Ok(());
    }

    let info = read_pid_file(&pid_file).context("Failed to read PID file")?;

    let has_socket = !info.socket_path.as_os_str().is_empty();
    let process_alive = is_process_running(info.pid);
    // Socket existence check is meaningful on Unix where the socket is a real
    // filesystem entry. Skip it on Windows where named pipes are not regular files.
    #[cfg(unix)]
    let socket_exists = !has_socket || info.socket_path.exists();
    #[cfg(windows)]
    let socket_exists = true;

    match (process_alive, socket_exists) {
        (true, true) => {
            println!("Status:   running");
            println!("PID:      {}", info.pid);
            if has_socket {
                println!("Socket:   {}", info.socket_path.display());
                println!();
                println!("To connect to the agent, set SSH_AUTH_SOCK:");
                #[cfg(unix)]
                println!("  export SSH_AUTH_SOCK={}", info.socket_path.display());
                #[cfg(windows)]
                println!("  $env:SSH_AUTH_SOCK={}", info.socket_path.display());
            }
        }
        (true, false) => {
            println!("Status:   degraded (process is running but socket is missing)");
            println!("PID:      {}", info.pid);
            println!("Socket:   {} (not found)", info.socket_path.display());
            println!();
            println!("Hint:     the agent process is alive but the socket file is gone.");
            println!("          Run 'ssh-agent daemon stop' then 'ssh-agent daemon start'.");
        }
        (false, true) => {
            println!("Status:   stopped (process died, stale socket file present)");
            println!("PID:      {} (not running)", info.pid);
            println!("Socket:   {} (stale)", info.socket_path.display());
            println!();
            println!("Hint:     run 'ssh-agent daemon start' to start the daemon.");
            println!("          The stale socket file will be cleaned up automatically.");
        }
        (false, false) => {
            println!("Status:   stopped");
            println!("PID:      {} (not running)", info.pid);
            if has_socket {
                println!("Socket:   {} (not found)", info.socket_path.display());
            }
            println!();
            println!("Hint:     run 'ssh-agent daemon start' to start the daemon.");
        }
    }

    println!("PID file: {}", pid_file.display());

    if let Some(ref log_path) = info.log_file {
        println!("Log file: {}", log_path.display());
        println!();
        match last_lines(log_path, 10) {
            Ok(lines) if lines.is_empty() => println!("(log file is empty)"),
            Ok(lines) => {
                println!("Last {} log line(s):", lines.len());
                for line in &lines {
                    println!("  {}", line);
                }
            }
            Err(e) => println!("(could not read log file: {})", e),
        }
    }

    Ok(())
}

fn last_lines(path: &Path, n: usize) -> Result<Vec<String>> {
    use std::io::{Read, Seek, SeekFrom};
    let mut file = std::fs::File::open(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    let file_len = file.seek(SeekFrom::End(0))?;
    let offset = file_len.saturating_sub(1024);
    file.seek(SeekFrom::Start(offset))?;
    let mut buf = String::new();
    file.read_to_string(&mut buf)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    let lines: Vec<&str> = buf.lines().collect();
    let start = lines.len().saturating_sub(n);
    Ok(lines[start..].iter().map(|s| s.to_string()).collect())
}

pub fn run_daemon_stop(pid_file: Option<PathBuf>) -> Result<()> {
    let pid_file = pid_file.map(Ok).unwrap_or_else(get_default_pid_file)?;

    if !pid_file.exists() {
        return Err(anyhow!(
            "Daemon is not running (no PID file at {})",
            pid_file.display()
        ));
    }

    let info = read_pid_file(&pid_file).context("Failed to read PID file")?;

    if !is_process_running(info.pid) {
        let _ = std::fs::remove_file(&pid_file);
        return Err(anyhow!("Daemon is not running (stale PID: {})", info.pid));
    }

    stop_process(info.pid).context("Failed to stop daemon process")?;
    let _ = std::fs::remove_file(&pid_file);
    println!("Daemon stopped (PID: {})", info.pid);

    Ok(())
}

// Configure `cmd` to run as a detached background process.
// Unix: call `setsid()` in the forked child before exec so the process
// becomes a session leader and is fully detached from the controlling
// terminal.
#[cfg(unix)]
fn platform_setup_daemon(cmd: &mut std::process::Command) {
    use std::os::unix::process::CommandExt;
    // SAFETY: pre_exec runs in the forked child before exec.
    // setsid() is async-signal-safe and is valid to call in this context.
    unsafe {
        cmd.pre_exec(|| {
            nix::unistd::setsid()
                .map(|_| ())
                .map_err(std::io::Error::from)
        });
    }
}

// Configure `cmd` to run as a detached background process.
// Windows: use `CREATE_NO_WINDOW | DETACHED_PROCESS` creation flags so the
// process has no console window and is not attached to the parent's console.
#[cfg(windows)]
fn platform_setup_daemon(cmd: &mut std::process::Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const DETACHED_PROCESS: u32 = 0x0000_0008;
    cmd.creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS);
}

// Returns `true` if a process with this PID is currently running.
// Unix: sends signal 0 (no-op probe) via `kill(2)`.
#[cfg(unix)]
fn is_process_running(pid: u32) -> bool {
    use nix::sys::signal;
    use nix::unistd::Pid;
    // Sending signal 0 only checks for process existence without side effects.
    signal::kill(Pid::from_raw(pid as i32), None).is_ok()
}

// Returns `true` if a process with this PID is currently running.
// Windows: queries the process list with `tasklist`.
#[cfg(windows)]
fn is_process_running(pid: u32) -> bool {
    std::process::Command::new("tasklist")
        .args(["/FI", &format!("PID eq {}", pid), "/NH", "/FO", "CSV"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains(&pid.to_string()))
        .unwrap_or(false)
}

// Terminates the process with the given PID.
// Unix: sends `SIGTERM`.
#[cfg(unix)]
fn stop_process(pid: u32) -> Result<()> {
    use nix::sys::signal::{self, Signal};
    use nix::unistd::Pid;
    signal::kill(Pid::from_raw(pid as i32), Signal::SIGTERM)
        .map_err(|e| anyhow!("Failed to send SIGTERM to PID {}: {}", pid, e))
}

// Terminates the process with the given PID.
// Windows: calls `taskkill /F`.
#[cfg(windows)]
fn stop_process(pid: u32) -> Result<()> {
    let status = std::process::Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/F"])
        .status()
        .context("Failed to run taskkill")?;
    if !status.success() {
        return Err(anyhow!("taskkill failed with status: {}", status));
    }
    Ok(())
}
