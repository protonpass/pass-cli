# Installation

This guide covers different ways to install the Proton Pass CLI on your system.

## Quick Install

The easiest way to install Proton Pass CLI is using the official installation script:

**macOS and Linux:**

```bash
curl -fsSL https://proton.me/download/pass-cli/install.sh | bash
```

**Windows:**

```powershell
Invoke-WebRequest -Uri https://proton.me/download/pass-cli/install.ps1 -OutFile install.ps1; .\install.ps1
```

The installation script will:

- Detect your operating system and architecture
- Download the latest stable release
- Verify the binary integrity
- Install the binary to a directory in your PATH (or prompt you to add it)
- Check for required system dependencies

## Homebrew (macOS only)

If you're on macOS and use Homebrew, you can install Proton Pass CLI via our official tap:

```bash
brew install protonpass/tap/pass-cli
```

To update to the latest version:

```bash
brew update
brew upgrade pass-cli
```

> [!WARNING]
> **Installing via Homebrew**
> When installed via Homebrew (or any package manager):
>
> - **The `pass-cli update` command will not work**: you must use `brew upgrade pass-cli` instead
> - **Track switching is not available**: you cannot switch between stable and beta tracks
> - Updates are managed entirely by Homebrew

## Installation options

### Custom installation directory

You can specify a custom installation directory:

**macOS and Linux:**

```bash
export PROTON_PASS_CLI_INSTALL_DIR=/custom/path
curl -fsSL https://proton.me/download/pass-cli/install.sh | bash
```

**Windows:**

```powershell
Invoke-WebRequest -Uri https://proton.me/download/pass-cli/install.ps1 -OutFile install.ps1
$env:PROTON_PASS_CLI_INSTALL_DIR="C:\custom\path"; .\install.ps1
```

### Beta channel

To install from the beta channel:

**macOS and Linux:**

```bash
export PROTON_PASS_CLI_INSTALL_CHANNEL=beta
curl -fsSL https://proton.me/download/pass-cli/install.sh | bash

# Or as a one-liner
curl -fsSL https://proton.me/download/pass-cli/install.sh | PROTON_PASS_CLI_INSTALL_CHANNEL=beta bash
```

**Windows:**

```powershell
Invoke-WebRequest -Uri https://proton.me/download/pass-cli/install.ps1 -OutFile install.ps1
$env:PROTON_PASS_CLI_INSTALL_CHANNEL="beta"; .\install.ps1
```

> [!NOTE]
> **Track Switching for Manual Installations**
> If you install the Pass CLI by selecting an install channel, you will automatically be switched to that release track. You can switch between tracks later or revert to the `stable` track using the [`update` command](../commands/update.md).
>
> **Important:** Track switching is only available for manual installations. If you install via a package manager, you cannot switch tracks.

## System requirements

### Supported platforms

- **macOS**: Intel (x86_64) and Apple Silicon (arm64)
- **Linux**: x86_64 and aarch64 architectures
- **Windows**: x86_64 architectures

### Dependencies

**macOS:**

- `curl` and `jq` for the installation script

**Linux:**

- `curl` and `jq` for the installation script

**Windows:**

- No extra dependencies are needed.

## Manual installation

If you prefer to install manually, you can download the binary listing file directly from:

```text
https://proton.me/download/pass-cli/versions.json
```

1. Download the versions listing file
2. Download the appropriate binary for your platform
3. (Optional but recommended): Verify the hash by running `sha256sum` on the binary you downloaded and compare it against the one listed in the versions listing file
4. Make it executable (on Unix systems): `chmod +x pass-cli`
5. Move it to a directory in your PATH (e.g., `/usr/local/bin` on Unix, or add to PATH on Windows)
6. Verify installation: `pass-cli --version`

## Verify installation

After installation, verify that the CLI is working:

**macOS/Linux**:

```bash
pass-cli --version
```

**Windows**:

```bash
pass-cli.exe --version
```

You should see the version number. If you get a "command not found" error, make sure the installation directory is in your PATH.

## Next steps

Once installed, proceed to the [Getting started](../getting-started/login.md) guide to learn how to authenticate and configure the CLI.
