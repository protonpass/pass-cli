# `login` command

Authenticate with Proton Pass and establish a session.

## Synopsis

```bash
pass-cli login [--interactive]
```

## Description

The `login` command authenticates you with Proton Pass. By default, it uses web-based authentication (prints a URL to complete the flow in your preferred browser). With the `--interactive` flag, it uses command-line authentication with username and password. Once authenticated, the session is stored locally and you can use other commands without re-authenticating.

## How it works

### Default mode (web login)

When run without `--interactive`, the command:

1. Prints a URL that you should open in your preferred browser
2. After successful authentication, stores the session locally
3. Creates a default vault named "Personal" if no vaults exist

### Interactive mode

When run with `--interactive`:

1. Prompts for username (or uses provided argument)
2. Prompts for password
3. Handles two-factor authentication (Only TOTP is supported. For FIDO2 flows please use the web login)
4. Handles extra password if required
5. Performs first-time setup
6. Creates a default vault named "Personal" if no vaults exist

## Arguments

- `USERNAME` (optional): Your Proton account email address. If not provided and `--interactive` is used, you'll be prompted.
- `--interactive`: Use command-line interactive authentication instead of web login.

## Mutually exclusive options

There are no mutually exclusive options for this command. The `--interactive` flag switches between web and command-line authentication modes.

## Authentication flow

### Interactive authentication

By default, the login process is interactive:

1. Enter your username as a command argument
2. You'll be prompted to enter your password
3. If two-factor authentication is enabled, you'll be prompted for your TOTP code
4. Upon successful authentication, your session is stored locally

### Non-interactive authentication

For automation and scripting, you can provide credentials via environment variables:

#### Password authentication

!!! warning "Password in env variables"
    Storing your password in an environment variable makes it readable by all other processes under the same
    session. Be conscious about doing so, and clear the variable when you are done.

```bash
# Provide password via environment variable
export PROTON_PASS_PASSWORD="your_password"
pass-cli login user@proton.me

# Or read password from a file
export PROTON_PASS_PASSWORD_FILE="/path/to/password/file"
pass-cli login user@proton.me
```

#### Two-factor authentication

If your account has 2FA enabled:

```bash
# Provide TOTP via environment variable
export PROTON_PASS_TOTP="123456"
pass-cli login user@proton.me

# Or read TOTP from a file
export PROTON_PASS_TOTP_FILE="/path/to/totp/file"
pass-cli login user@proton.me
```

#### Combined non-interactive login

```bash
export PROTON_PASS_PASSWORD="your_password"
export PROTON_PASS_TOTP="123456"
pass-cli login user@proton.me
```

## Examples

### Basic interactive login

```bash
pass-cli login alice@proton.me
# You'll be prompted for password and TOTP (if enabled)
```

### Automated login script

```bash
#!/bin/bash
export PROTON_PASS_PASSWORD="$(cat ~/.proton-pass-password)"
export PROTON_PASS_TOTP="$(generate-totp-code)"
pass-cli login alice@proton.me
```

### Reading credentials from files

```bash
# Store password in a secure file
echo "your_password" > ~/.proton-pass-password
chmod 600 ~/.proton-pass-password

# Configure environment
export PROTON_PASS_PASSWORD_FILE="$HOME/.proton-pass-password"
pass-cli login alice@proton.me
```

## Security considerations

- **Password storage**: Never store passwords in plain text files or environment variables in production
- **TOTP codes**: TOTP codes are time-sensitive and expire quickly
- **Session storage**: Authenticated sessions are stored locally and persist until logout
- **File permissions**: Ensure credential files have restrictive permissions (600)

## Supported authentication methods

- **Username/Password**: Standard authentication
- **Two-Factor Authentication (TOTP)**: Time-based one-time passwords
- **FIDO authentication**: Currently not supported

## Session management

After successful login:

- Your session is stored in a platform-specific location
- The session persists across command invocations
- Use `pass-cli logout` to end the session
- Use `pass-cli test` to verify your session is valid

## Troubleshooting

### Login failures

- **Invalid credentials**: Double-check your username and password
- **2FA required**: Ensure you're providing TOTP when required
- **Network issues**: Check your internet connection
- **Account locked**: Contact Proton support if your account is locked

### Environment variable issues

- **File not found**: Ensure password/TOTP files exist and are readable
- **Permission denied**: Check file permissions for credential files
- **Variable not set**: Verify environment variables are properly exported
