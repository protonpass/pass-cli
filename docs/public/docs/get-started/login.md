# Login and authentication

This section explains how to authenticate with the Proton Pass CLI, including all available options for providing credentials.

## Web login

To log in with your Proton account:

```bash
pass-cli login
```

Then, open the URL that you will see in your web browser, complete the authentication flow, and your Proton Pass CLI will be automatically logged in.

## Interactive login

!!! warning "Interactive login restrictions"
    Take into account that not all login flows are supported in the Interactive login. SSO login flows, or 2FA requiring
    a U2F key are only supported in the web login.

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

## Providing credentials

For each authentication parameter, the CLI checks for values in this order:

1. **Environment variable** - Direct value
2. **File referenced by environment variable** - Path to file containing the value
3. **Interactive prompt** - If not found in env vars, prompts the user

### Password

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

To log out and clear your session:

```bash
pass-cli logout
```

To force logout even if remote logout fails:

```bash
pass-cli logout --force
```

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

