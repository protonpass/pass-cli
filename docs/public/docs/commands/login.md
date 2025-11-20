# `login` command

Authenticate with Proton Pass and establish a session.

## Synopsis

```bash
pass-cli login [--interactive]
```

## Description

The `login` command authenticates you with Proton Pass. By default, it uses web-based authentication (prints a URL to complete the flow in your preferred browser). With the `--interactive` flag, it uses command-line authentication with username and password. Once authenticated, the session is stored locally and you can use other commands without re-authenticating.

## Web login

The easiest way to log into your Proton account in the Proton Pass CLI is to perform the login via web. In order to do so, run the following command:

```bash
pass-cli login
```

Then, open the URL that you will see in your web browser, complete the authentication flow, and your Proton Pass CLI will be automatically logged in.

In case you want to log in by using SSO, or your account is protected by a hardware key, the web login is the only supported login flow.

## Interactive login

!!! warning "Interactive login restrictions"
    Take into account that not all login flows are supported in the Interactive login. SSO login flows, or 2FA requiring
```
a U2F key are only supported in the web login.
```

To log in with your Proton account directly in the CLI:

```bash
pass-cli login --interactive [USERNAME]
```

The authentication flow will then happen entirely in your terminal.

### Interactive authentication flow

The login process follows these steps:

1. **Password authentication** - You'll be prompted for your Proton account password
2. **Two-factor authentication** (if enabled) - You'll be prompted for your TOTP token
3. **Extra password** (if required) - Proton Pass users can configure their accounts to require an additional Pass-specific password
4. **Initial setup** - The CLI performs first-time setup and creates a default vault named "Personal" if none exists
5. **Permission check** - Verifies that your account is authorized to use the CLI

### Providing credentials

For each authentication parameter, the CLI checks for values in this order:

1. **Environment variable** - Direct value
2. **File referenced by environment variable** - Path to file containing the value
3. **Interactive prompt** - If not found in env vars, prompts the user

### Password

!!! warning "Password in env variables"
    Storing your password in an environment variable makes it readable by all other processes under the same
```
session. Be conscious about doing so, and clear the variable when you are done.
```

**Interactive (default):**

```bash
pass-cli login --interactive user@proton.me
# You will be prompted: Enter password:
```

**Via environment variable:**

```bash
export PROTON_PASS_PASSWORD='your-password'
pass-cli login --interactive user@proton.me
```

**Via file:**

```bash
echo 'your-password' > /secure/password.txt
export PROTON_PASS_PASSWORD_FILE='/secure/password.txt'
pass-cli login --interactive user@proton.me
```

### Two-factor authentication (TOTP)

If your account has TOTP enabled:

**Interactive (default):**

If not supplied in any other way, after the password step, you'll be prompted for your TOTP.

```bash
pass-cli login --interactive user@proton.me
# Enter password:
# Enter TOTP:
```

**Via environment variable:**

```bash
export PROTON_PASS_TOTP='123456'
pass-cli login --interactive user@proton.me
```

**Via file:**

```bash
echo '123456' > /secure/totp.txt
export PROTON_PASS_TOTP_FILE='/secure/totp.txt'
pass-cli login --interactive user@proton.me
```

### Extra password

Some Proton Pass accounts require an additional password (separate from your account password). This is the Pass-specific access password.

**Interactive (default):**

If not supplied in any other way, after the password step (and optionally the TOTP step), you'll be prompted for your Proton Pass extra password.

```bash
pass-cli login --interactive user@proton.me
# Enter password:
# (Optional) Enter TOTP:
# Enter Pass extra password:
```

**Via environment variable:**

```bash
export PROTON_PASS_EXTRA_PASSWORD='your-extra-password'
pass-cli login --interactive user@proton.me
```

**Via file:**

```bash
echo 'your-extra-password' > /secure/extra-password.txt
export PROTON_PASS_EXTRA_PASSWORD_FILE='/secure/extra-password.txt'
pass-cli login --interactive user@proton.me
```

You have 3 attempts to enter the correct extra password before the CLI logs out.

## Checking authentication status

After logging in, verify your session:

```bash
pass-cli info
```

This shows:

- Your username
- Account details
- Active session information

## Logout

To log out from your session, please check the [logout command](logout.md).

## Example: Fully automated login

For scripting and automation, you can provide all credentials via environment variables:

```bash
#!/bin/bash

export PROTON_PASS_PASSWORD='your-password'
export PROTON_PASS_TOTP='123456'
export PROTON_PASS_EXTRA_PASSWORD='your-extra-password'

pass-cli login --interactive user@proton.me
```

Or using files:

```bash
#!/bin/bash

export PROTON_PASS_PASSWORD_FILE='/secure/creds/password.txt'
export PROTON_PASS_TOTP_FILE='/secure/creds/totp.txt'
export PROTON_PASS_EXTRA_PASSWORD_FILE='/secure/creds/extra-password.txt'

pass-cli login --interactive user@proton.me
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
- **FIDO authentication**: Only supported in web login

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
