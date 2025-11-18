# Installation

This guide covers different ways to install the Proton Pass CLI on your system.

## Quick Install

The easiest way to install Proton Pass CLI is using the official installation script:

**macOS and Linux:**

```bash
curl -fsSL https://proton.me/download/pass-cli/install.sh | bash
```

**Windows:**

> A native Windows version is not yet available. See the [Windows (WSL) Installation](#windows-wsl-installation) section below for a workaround using Windows Subsystem for Linux.

The installation script will:

- Detect your operating system and architecture
- Download the latest stable release
- Verify the binary integrity
- Install the binary to a directory in your PATH (or prompt you to add it)
- Check for required system dependencies

## Installation Options

### Custom Installation Directory

You can specify a custom installation directory:

**macOS and Linux:**

```bash
export PROTON_PASS_CLI_INSTALL_DIR=/custom/path
curl -fsSL https://proton.me/download/pass-cli/install.sh | bash
```


### Beta Channel

To install from the beta channel:

**macOS and Linux:**

```bash
export PROTON_PASS_CLI_INSTALL_CHANNEL=beta
curl -fsSL https://proton.me/download/pass-cli/install.sh | bash
```


## System Requirements

### Supported Platforms

- **macOS**: Intel (x86_64) and Apple Silicon (arm64)
- **Linux**: x86_64 and aarch64 architectures
- **Windows**: Not yet available natively (use [WSL](#windows-wsl-installation) as a workaround)

### Dependencies

**macOS:**

- No additional dependencies required

**Linux:**

- `curl` and `jq` for the installation script
- System libraries: `libdbus-1-3` or `dbus-libs` (for keyring support)

**Windows:**

- Not yet available natively. See [Windows (WSL) Installation](#windows-wsl-installation) below.

## Windows (WSL) Installation

While a native Windows version is not yet available, you can use Windows Subsystem for Linux (WSL) to run the Linux version of Proton Pass CLI.

### Step 1: Install WSL

If you don't have WSL installed, follow these steps:

1. Open PowerShell or Windows Command Prompt in **Administrator mode** by right-clicking and selecting "Run as administrator"

2. Install WSL with the default Ubuntu distribution:

```powershell
wsl --install
```

3. Restart your computer when prompted

4. After restart, Ubuntu will open automatically and prompt you to create a username and password

**Alternative: Install a Specific Distribution**

To see available Linux distributions:

```powershell
wsl --list --online
```

To install a specific distribution (e.g., Ubuntu 22.04):

```powershell
wsl --install -d Ubuntu-22.04
```

### Step 2: Install Proton Pass CLI in WSL

Once WSL is set up:

1. Open your WSL terminal (search for "Ubuntu" or "WSL" in the Start menu)

2. Update your package manager:

```bash
sudo apt update && sudo apt upgrade -y
```

3. Install required dependencies:

```bash
sudo apt install -y curl jq libdbus-1-3
```

4. Install Proton Pass CLI using the installation script:

```bash
curl -fsSL https://proton.me/download/pass-cli/install.sh | bash
```

5. Verify the installation:

```bash
pass-cli --version
```

### Accessing WSL from Windows

- Open WSL terminal by searching for "Ubuntu" or "WSL" in the Start menu
- Access your Windows files from WSL at `/mnt/c/` (C: drive), `/mnt/d/` (D: drive), etc.
- Access your WSL files from Windows at `\\wsl$\Ubuntu\home\<username>\`

### Additional Notes

- You can run `pass-cli` commands directly from PowerShell/CMD using: `wsl pass-cli <command>`
- For the best experience, we recommend using Windows Terminal (available from the Microsoft Store)

## Manual Installation

If you prefer to install manually, you can download the binary directly from:

```text
https://proton.me/download/pass-cli/
```

1. Download the appropriate binary for your platform
2. Make it executable (on Unix systems): `chmod +x pass-cli`
3. Move it to a directory in your PATH (e.g., `/usr/local/bin` on Unix, or add to PATH on Windows)
4. Verify installation: `pass-cli --version`

## Building from Source

If you want to build from source, you'll need:

### Prerequisites

- **Rust toolchain** (1.89 or later recommended)
- **Cargo** (comes with Rust)

**On macOS:**

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

**On Linux (Ubuntu/Debian):**

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
sudo apt-get update
sudo apt-get install -y pkg-config libssl-dev libdbus-1-dev
```

**On Linux (RHEL/Fedora):**

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
sudo dnf install -y pkg-config openssl-devel dbus-devel systemd-devel
```

### Build

Clone the repository and build:

```bash
git clone <repository-url>
cd proton-pass-cli
cargo build --release
```

The binary will be located at `target/release/pass-cli`.

### Build Features

The project supports several build features:

- `keyring` (enabled by default) - System keyring integration for secure key storage
- `internal` - Internal testing commands (not recommended for production)
- `no-login-restriction` - Disables login restrictions for testing

To build with specific features:

```bash
cargo build --release --features no-login-restriction --features keyring
```

## Verify Installation

After installation, verify that the CLI is working:

```bash
pass-cli --version
```

You should see the version number. If you get a "command not found" error, make sure the installation directory is in your PATH.

## Next Steps

Once installed, proceed to the [Getting Started](../getting-started/login.md) guide to learn how to authenticate and configure the CLI.
