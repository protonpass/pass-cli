# `logout` command

End the current Proton Pass session and remove all local data.

## Synopsis

```bash
pass-cli logout [--force]
```

## Description

The `logout` command terminates your current Proton Pass session and removes all locally stored authentication data, including session tokens and encryption keys. After logging out, you'll need to authenticate again with `pass-cli login` to use other commands.

## How it works

1. **Remote logout**: Attempts to invalidate the session on the Proton Pass servers
2. **Key removal**: Removes the local encryption key from the keyring or filesystem
3. **Data cleanup**: Deletes all local session data and cached information
4. **Error handling**: If remote logout fails, you can use `--force` to proceed with local cleanup only

## Arguments

- `--force`: Force logout even if remote logout fails. This removes all local data without attempting to invalidate the session on the server.

## Mutually exclusive options

There are no mutually exclusive options for this command.

## Examples

### Basic logout

```bash
pass-cli logout
# Successfully logged out
```

### Force logout

If remote logout fails (e.g., network issues), you can force local cleanup:

```bash
pass-cli logout --force
# Executing force logout
# Successfully performed force logout
```

### Logout in scripts

```bash
#!/bin/bash
# Perform operations
pass-cli vault list
pass-cli item list

# Clean up session when done
pass-cli logout
```

## Security considerations

- **Complete cleanup**: Logout ensures no authentication data remains on the system
- **Shared systems**: Always logout when using shared or public computers
- **Automation**: Include logout in automated scripts to prevent session leakage
- **Session isolation**: Each login creates a fresh session

## When to logout

### Required scenarios
- **Shared computers**: Always logout on public or shared systems
- **Security protocols**: When organizational policy requires session cleanup
- **Account switching**: Before logging in with a different account

### Recommended scenarios
- **End of work session**: When finishing work for the day
- **Automated scripts**: At the end of automation scripts
- **Troubleshooting**: When experiencing authentication issues


## Session persistence

Without logout:
- Sessions remain active across terminal sessions
- Authentication persists until explicitly ended
- Session data is stored securely on the local system
- Sessions may have server-side expiration policies

## Troubleshooting

### Logout issues

If logout fails or behaves unexpectedly:

```bash
# Force logout by removing local data manually
# (This is a last resort - normal logout should work)
rm -rf ~/.config/pass-cli/  # Linux/macOS
```

### Verification

To verify logout was successful:

```bash
# This should require authentication
pass-cli test
# Expected: Error indicating authentication is required
```

## Related commands

- **[login](login.md)** - Authenticate and start a new session
- **[test](test.md)** - Verify current authentication status
- **[info](info.md)** - Display current session information
