# `session` command

Manage the session lock to prevent unauthorized access to your Proton Pass CLI session with a lock code code.

## Synopsis

```bash
pass-cli session create-lock [--idle-timeout SECONDS]
pass-cli session lock
pass-cli session unlock
pass-cli session remove-lock
```

## Description

The `session` command lets you add a lock code-based lock to your active session. When the session is locked, all operations that require the Proton Pass API are blocked until you unlock it with the correct lock code. This is useful when you want to keep your session authenticated but prevent anyone with access to your terminal from running commands. The lock is enforced server-side: even if local state is tampered with, the Proton Pass API will reject requests until the session is unlocked. The lock also auto-expires after the configured timeout, at which point the session becomes unusable again without a lock code.

The session lock is only available for standard user sessions. PAT sessions and agent sessions cannot have a session lock, and they have a lifetime of 2 hours.

## Subcommands

### create-lock

Create a lock for the current session with a lock code.

```bash
pass-cli session create-lock [--idle-timeout SECONDS]
```

You will be prompted to enter a lock code. The lock code is not stored anywhere, it is sent to the Proton Pass API to establish the lock. You must use the same lock code to unlock or remove the lock later.

**Options:**

- `--idle-timeout SECONDS` Time in seconds before the session auto-unlocks. Must be between 30 and 900. Default: `300` (5 minutes).
 
**Examples:**

```bash
# Lock with the default 5-minute timeout
pass-cli session create-lock
# Enter lock code:
# Session locked successfully
# Lock with a custom 10-minute timeout
pass-cli session create-lock --idle-timeout 600
# Enter lock code:
# Session locked successfully
```

---

### lock

Lock the current session immediately, without waiting for the idle timeout to elapse.

```bash
pass-cli session lock
```

This requires a lock to already exist (created with `create-lock`). If the session has no lock, the command fails and asks you to create one first. After locking, all operations that require the Proton Pass API are blocked until you unlock the session with the correct lock code.

**Examples:**

```bash
pass-cli session lock
# Session locked successfully
```

---

### unlock

Unlock a locked session using the lock code set at lock time.

```bash
pass-cli session unlock
```

You will be prompted for the lock code. On success, the session is restored to normal operation. This command fails if the session is not currently locked.

**Examples:**

```bash
pass-cli session unlock
# Enter lock code:
# Session unlocked successfully
```

---

### remove-lock

Remove the session lock entirely, so no lock code is required going forward.

```bash
pass-cli session remove-lock
```

You will be prompted for the current lock code to confirm the removal. After this, the lock is deleted from the server and the session operates normally without any lock code requirement.

**Examples:**

```bash
pass-cli session remove-lock
# Enter lock code:
# Session lock removed successfully
```

## Checking lock status

```bash
pass-cli info
```

The output includes a `Session has lock` field that shows whether the current session has an active lock. Having a session lock does not mean that the session is locked at this moment. It means that if unused it will lock automatically.

## Security considerations

- **lock code strength** Choose a lock code that is not trivially guessable. There is no minimum length enforced by the CLI, but a longer lock code is harder to brute-force.
- **Auto-unlock timeout** Keep `--idle-timeout` short on shared or unattended systems. The default of 300 seconds is a reasonable balance for interactive use.
- **Session vs. logout** Locking a session is not a substitute for `logout`. A locked session is still authenticated; it is just gated by the lock code. Use `logout` when you want to fully terminate the session.
- **Lock code not stored** The lock code is never written to disk or the keyring. If you forget it, you cannot unlock or remove the lock until it auto-expires. You will need to log out and log in again.
- **ssh-agent** The `ssh-agent` commands will not refresh the keys automatically nor create items directly from ssh if the session has a lock.
