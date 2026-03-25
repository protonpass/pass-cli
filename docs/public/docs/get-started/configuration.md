# Configuration

This section defines which parts of the CLI are configurable and how to do so.

## Personal access tokens

Personal access tokens are a way to authenticate with Proton Pass without using your full account credentials. Instead of logging in as your user account, you authenticate as a named token that you control, with access restricted to exactly the vaults and items you choose.

They're particularly useful for the following scenarios:

- **Automated executions**: Automate secret retrieval without embedding your account password. For instance when running in a CI/CD pipeline, cron jobs, deployment scripts..
- **Restricted access control**: Grant limited permissions to PAT to reduce the impact if the token is compromised.
- **Headless or remote environments**: Run `pass-cli` in a container or headless server where you cannot do web login.
- **Temporary access**: Limit the time the access is granted to a PAT without affecting your main account.

### Access control

When you create a personal access token, it has no access to anything until you explicitly grant it. You can grant access at two levels:

- **Vault level** - the token can read (or write) all items in the vault
- **Item level** - the token can only access a specific item, even if it's inside a shared vault

Each grant also carries a role (`viewer`, `editor`, or `manager`), so you can further limit what the token can do with the items it can see.

Tokens also have a mandatory expiration date, so they stop working automatically after a set period. This limits the damage if a token is ever leaked.

### Logging in with a personal access token

There are two ways to provide the token when logging in.

**Via environment variable (recommended for CI):**

```bash
PROTON_PASS_PERSONAL_ACCESS_TOKEN=pst_xxxx...xxxx::TOKENKEY pass-cli login
```

This keeps the token out of your shell history and is the easiest option for automated environments.

**Via command-line flag:**

```bash
pass-cli login --personal-access-token "pst_xxxx...xxxx::TOKENKEY"
```

Once logged in, the CLI behaves normally. Commands like `pass-cli info` will show the token name under "Personal Access Token" instead of a user email.

For instructions on creating and managing tokens, see the [`pat` command reference](../commands/personal-access-token.md).

---

## Configuring logging

Control log output verbosity with this environment variable:

**Pass CLI logging:**

```bash
# Levels: trace, debug, info, warn, error, off
export PASS_LOG_LEVEL=debug
```

Take into account that CLI logs are sent to `stderr`, so they should not interfere with any piping / command integration you are using.

## Session storage directory

By default, session data is stored in:

- **macOS**: `~/Library/Application Support/proton-pass-cli/.session/`
- **Linux**: `~/.local/share/proton-pass-cli/.session/`

If desired, you can override this with:

```bash
export PROTON_PASS_SESSION_DIR='/custom/path'
```

## Secure key storage

The CLI supports two key storage backends, controlled by the `PROTON_PASS_KEY_PROVIDER` environment variable:

### 1. Keyring storage (default)

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

On Linux, the CLI defaults to the **kernel keyring** (kernel key retention service), which:

- Stores the encryption key in kernel memory
- Does not require D-Bus or a graphical session
- **Secrets are cleared on system reboot**

If you have a desktop environment with a Secret Service provider (e.g. GNOME Keyring), you can opt in to D-Bus-backed persistent storage instead:

```bash
export PROTON_PASS_LINUX_KEYRING=dbus
```

| Value | Backend | Persists across reboots | Requires D-Bus |
|-------|---------|------------------------|----------------|
| `kernel` (default) | Kernel key retention service | No | No |
| `dbus` | D-Bus Secret Service (e.g. GNOME Keyring) | Yes | Yes |

When `dbus` is set and the Secret Service is unavailable or locked (e.g. the GNOME Keyring has not been unlocked yet), the CLI will fail with an error rather than falling back silently. Unlock your desktop session first, then retry.

For the GNOME Keyring, if you don't have physical access to the device, you can use the following script:

```bash
function unlock-keyring ()
{
    read -rsp "Password: " keyringpass
    export $(echo -n "$keyringpass" | gnome-keyring-daemon --replace --unlock)
    unset keyringpass
}

unlock-keyring
```

Take into account that when running in docker containers, the container cannot access the kernel secret service, so the only option available to be used when running in a container is the Filesystem storage.

### 2. Filesystem storage

!!! warning "Using the key filesystem storage"
    Take into account that storing your key in the local filesystem makes the encryption key be side-by-side with the
    encrypted data, which could make it easier for an attacker to get access to your data. By using this option you are
    in charge of securing access to your system and your data.

**Configuration:**

```bash
export PROTON_PASS_KEY_PROVIDER=fs
```

This stores the encryption key in a file on disk:

```text
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

### 3. Environment variable storage

!!! warning "Using the environment variable storage"
    Take into account that storing your key in an environment variable makes it available to any other process that is under the same session / in the same container.
    By using this option you are in charge of securing access to your system and your data.

**Configuration:**

```bash
export PROTON_PASS_KEY_PROVIDER=env
export PROTON_PASS_ENCRYPTION_KEY=your-secret-key
```

This derives the encryption key from the `PROTON_PASS_ENCRYPTION_KEY` environment variable, which **must be set and non-empty**.

If you are running linux or macOS, you can easily generate a safe encryption key by executing:

```bash
dd if=/dev/urandom bs=1 count=2048 2>/dev/null | sha256sum | awk '{print $1}'
```

**How it works:**

1. Read the `PROTON_PASS_ENCRYPTION_KEY` environment variable (must be set and non-empty, otherwise it will error)
2. Hash the value with SHA256 to get a consistent 256-bit key

While not in use, the encryption key is obfuscated in memory, and only decrypted when it needs to be used.

**Security properties:**

- Key is derived from a user-provided value
- Hashed with SHA256 for consistency and to ensure proper key length

**Advantages:**

- Portable across all environments
- No dependency on filesystem or system keyring
- User has full control over the key value
- Works in containers, CI/CD, and headless environments

**Disadvantages:**

- Less secure than OS keyring solutions
- Environment variables can be visible to other processes
- Key security depends entirely on how the environment is managed

**When to use environment variable storage:**

- CI/CD pipelines and automation
- Containerized environments where filesystem persistence is undesirable
- Scripts where the key can be securely injected
- When you need explicit control over the encryption key

