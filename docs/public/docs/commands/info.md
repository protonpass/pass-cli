# `info` command

Display information about the current Proton Pass session and user account.

## Synopsis

```bash
pass-cli info
```

## Description

The `info` command displays information about your current Proton Pass session, including user account details, release track, and environment information. This command is useful for verifying your login status and understanding your CLI configuration.

## How it works

The command retrieves user information from the Proton Pass API and displays:
- User ID
- Username
- Email address
- Release track (stable, beta, etc.)
- Environment (only shown if not production)

## Arguments

This command takes no arguments.

## Information displayed

The command displays:

- **ID**: Your Proton Pass user ID
- **Username**: Your Proton account username
- **Email**: Your Proton account email address
- **Release track**: The release track you're using (stable, beta, etc.)
- **ENV**: The environment (only shown if not production)

## Examples

### Basic information display

```bash
pass-cli info
- ENV: Production
- ID: YOUR_USER_ID_HERE
- Username: yourusername
- Email: yourusername@proton.me
```

### Troubleshooting workflow

```bash
echo "=== Session Information ==="
pass-cli info

echo "=== Connection Test ==="
pass-cli test

echo "=== Available Vaults ==="
pass-cli vault list
```

## Use cases

### Session verification

Verify you're logged in as the expected user:

```bash
pass-cli info | grep "Username:"
```

### Script validation

Validate session state before performing operations:

```bash
#!/bin/bash
if pass-cli info > /dev/null 2>&1; then
    echo "Session valid, continuing..."
    # Perform operations
else
    echo "Invalid session, please login"
    exit 1
fi
```

## Troubleshooting with info

### Authentication issues

If `pass-cli info` fails:
1. You're not logged in - use `pass-cli login`
2. Session expired - re-authenticate
3. Network issues - check connectivity

### Unexpected user

If the wrong user is shown:
1. Logout: `pass-cli logout`
2. Login with correct account: `pass-cli login correct@email.com`

## Privacy considerations

The `info` command shows information that is mainly safe to display to the operator, such as the account email and username. Sensitive info, such as passwords, private key or any user data is never shown.

## Related commands

- **[login](login.md)** - Authenticate and create a session
- **[logout](logout.md)** - End current session
- **[test](test.md)** - Test connection and authentication
- **[user info](user.md#info)** - More detailed user account information
