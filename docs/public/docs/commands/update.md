# `update` command

Update the Proton Pass CLI to the latest version and manage release tracks.

## Synopsis

```bash
pass-cli update [--yes] [--set-track TRACK]
```

## Description

The `update` command keeps your Proton Pass CLI up to date with the latest features, improvements, and security patches. It automatically downloads and installs the newest version available for your platform. You can also use this command to switch between different release tracks (stable and beta).

!!! warning "Manual installation only"
    **The `update` command and track switching only work if you installed Proton Pass CLI manually** (using the installation script or manual download). If you installed via a package manager (e.g., Homebrew), you must use that package manager's update mechanism instead. Track switching is not available for package manager installations.

## How to update

To update the CLI to the latest version:

```bash
pass-cli update
```

The command will:

1. Check for the latest version available on your current release track
2. Prompt you to confirm the update
3. Download the new binary and verify the checksums
4. Replace your current installation with the new version

### Automatic update (skip confirmation)

To update without confirmation prompts (useful for scripts):

```bash
pass-cli update --yes
```

## Release tracks

The Proton Pass CLI supports different release tracks, allowing you to choose between stability and early access to new features.

!!! warning "Track switching availability"
    Release track switching is **only available for manual installations**. If you installed Proton Pass CLI via a package manager (e.g., Homebrew), you cannot switch tracks using the `--set-track` option.

### Available tracks

- **stable**: The default track with thoroughly tested releases (recommended for production use)
- **beta**: Early access to new features before they reach stable

### Changing tracks

To switch to a different release track:

```bash
pass-cli update --set-track TRACK
```

#### Switch to beta track

Get early access to new features:

```bash
pass-cli update --set-track beta
```

After switching tracks, run the update command again to get the latest version from that track:

```bash
pass-cli update
```

#### Revert to stable track

You can switch back to the stable track at any time:

```bash
pass-cli update --set-track stable
pass-cli update
```

Your track preference is saved and persists across all future updates until you change it again.

## Checking your current track

To see which release track you're currently on, use the `info` command:

```bash
pass-cli info
```

The output will include your current release track:

```bash
- Release track: stable
- ID: YOUR_USER_ID
- Username: your-username
- Email: youruser@proton.me
```

## Automatic update checks

The Proton Pass CLI automatically checks for updates every 3 days. When a new version is available, you'll see an informative message like:

```
New update available: v1.0.0 -> v1.1.0 (run "pass-cli update")
```

### Important notes about automatic checks

- **Non-intrusive**: The update notification will not interrupt or disturb your workflow
- **Informative only**: It's just a friendly reminder that a new version is available
- **No forced updates**: You can continue using your current version
- **Background check**: The check happens automatically and doesn't slow down your commands

### Disabling automatic checks

If you prefer to check for updates manually, you can disable automatic checks:

```bash
export PROTON_PASS_NO_UPDATE_CHECK=1
```

## Arguments

- `--yes`: Skip confirmation prompt and update immediately
- `--set-track TRACK`: Change the release track (stable or beta)

## Examples

### Basic update

```bash
pass-cli update
# Update pass-cli v1.0.0 → v1.1.0? [Y/n]
# Downloading pass-cli v1.1.0...
# Installing...
# Updated to v1.1.0.
```

### Update without confirmation

```bash
pass-cli update --yes
# Downloading pass-cli v1.1.0...
# Installing...
# Updated to v1.1.0.
```

### Switch to beta track

```bash
# Set the track
pass-cli update --set-track beta
# Update track set to beta

# Update to latest beta version
pass-cli update
# Update pass-cli v1.0.0 → v1.1.0-beta.1? [Y/n]
```

### Check current version and track

```bash
pass-cli info
# - Release track: beta
# - ID: YOUR_USER_ID
# - Username: your-username
# - Email: youruser@proton.me
```

### Return to stable releases

```bash
# Switch back to stable
pass-cli update --set-track stable
# Update track set to stable

# Update to latest stable version
pass-cli update
# Update pass-cli v1.1.0-beta.1 → v1.0.5? [Y/n]
```

