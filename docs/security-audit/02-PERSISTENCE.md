# Data persistence and local storage

This document describes what data is persisted to disk by the Proton Pass CLI and how local encryption keys are managed.

## Overview

The CLI stores three types of data locally:

1. **Session data** - Authentication tokens, environment configuration and key passphrases
2. **Local encryption key** - Used to encrypt the session data at rest

All session data is encrypted before being written to disk.

## Session storage

### Location

Session data is stored in a platform-specific directory:

- **macOS**: `~/Library/Application Support/proton-pass-cli/.session/`
- **Linux**: `~/.local/share/proton-pass-cli/.session/`

This location can be overridden with the `PROTON_PASS_SESSION_DIR` environment variable:

```bash
export PROTON_PASS_SESSION_DIR='/custom/path'
```

### Session file

The session data is stored in a single file:

```
<session-dir>/session.json
```

This file contains:
- **Authentication tokens** - API access and refresh tokens
- **Environment information** - Which Proton API environment you're connected to (prod, custom)

### Encryption

The `session.json` file is **always encrypted** before being written to disk. The encryption process:

1. Serialize session data to JSON
2. Fetch the local encryption key (see below)
3. Encrypt the JSON using AES-256-GCM
4. Write the encrypted data to `session.json`

When reading the session:
1. Read encrypted data from `session.json`
2. Fetch the local encryption key
3. Decrypt the data
4. Deserialize from JSON

If decryption fails (e.g., key was changed or lost), the CLI will refuse to start and suggest logging out and back in.

## Local encryption key management

The local encryption key is a 256-bit symmetric key used exclusively to encrypt session data at rest. It never leaves your machine.

### Key providers

The CLI supports two key storage backends, controlled by the `PROTON_PASS_KEY_PROVIDER` environment variable:

#### 1. Keyring storage (default)

**Configuration:**

```bash
export PROTON_PASS_KEY_PROVIDER=keyring  # or unset
```

This uses the operating system's secure credential storage:

- **macOS**: macOS Keychain
- **Linux**: Kernel-based secret storage (by using the `linux_keyutils` crate via `keyring`)
- **Windows**: Windows Credential Manager

**How it works:**
1. On first run, generate a random 256-bit key
2. Store it in the system keyring
3. On subsequent runs, retrieve it from the keyring
4. If keyring is unavailable but session exists, force logout for security

**Linux keyring note:**

On Linux, using a keyring integration that uses D-Bus to communicate with the Secret Service API would be preferred. However, **D-Bus is not available in headless environments** (containers, SSH sessions without X11, distroless images, etc.).

To work around this limitation, the Linux keyring library is configured to store secrets in the **kernel keyring** (kernel key retention service). This approach:

- Stores the encryption key in kernel memory
- Does not require D-Bus or a graphical session
- **Secrets are cleared on system reboot**

This is a known limitation when running in headless Linux environments.

#### 2. Filesystem storage

**Configuration:**

```bash
export PROTON_PASS_KEY_PROVIDER=fs
```

This stores the encryption key in a file on disk:

```
<session-dir>/local.key
```

**How it works:**

1. On first run, generate a random 256-bit key
2. Write it to `local.key` with permissions `0600` (owner read/write only)
3. On subsequent runs, read the key from this file

**Security properties:**

- File permissions restrict access to the current user
- Key is stored in plaintext (though only readable by owner)
- Survives system reboots
- Does not require system keyring support

**Advantages:**

- Works in all environments (headless, containers, etc.)
- Survives reboots
- No dependency on system services

**Disadvantages:**

- Key is stored in plaintext on disk side-by-side with the encrypted data
- Less secure than OS keyring solutions
- If someone gains root access, they can read the key

**When to use filesystem storage:**

- Running in Docker containers
- Development/testing environments
- When system keyring is unavailable

## Security considerations

### Session security

The session file is always encrypted with a strong symmetric key. Even if an attacker obtains `session.json`, they cannot decrypt it without the local encryption key.

## Clearing stored data

To completely remove all local data:

```bash
# Log out (clears session)
pass-cli logout

# Remove session directory
rm -rf ~/Library/Application\ Support/proton-pass-cli/.session/  # macOS
rm -rf ~/.local/share/proton-pass-cli/.session/                 # Linux

# If using keyring, remove the key from system keychain manually
# macOS: Use Keychain Access app, search for "ProtonPassCLI"
# Linux: Depends on your keyring implementation
```

## Troubleshooting

### "Error decrypting local session"

This error occurs when:
- The local encryption key has changed
- The key storage backend was switched
- The key was manually deleted from the keyring

**Solution:**

```bash
pass-cli logout --force
pass-cli login <username>
```

### "Error accessing key provider"

This error occurs when the keyring is unavailable.

**Solution:**

Either try to figure out why the keyring provider is unavailable, or switch to filesystem key provider:

```bash
export PROTON_PASS_KEY_PROVIDER=fs
pass-cli login <username>
```

