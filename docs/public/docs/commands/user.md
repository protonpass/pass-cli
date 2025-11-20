# `user` command

Manage user account information and operations.

## Synopsis

```bash
pass-cli user <SUBCOMMAND>
```

## Description

The `user` command provides operations for viewing your Proton Pass user account information, including profile details, account status, and user-specific settings.

## Subcommands

### info

Display detailed information about your user account.

```bash
pass-cli user info [--output FORMAT]
```

**Options:**

- `--output FORMAT` - Output format: `human` (default) or `json`

**Examples:**

```bash
# Display user information in human-readable format
pass-cli user info

# Display user information in JSON format
pass-cli user info --output json
```

## User information

The `user info` command typically displays:

### Account details

- **Email address**: Your Proton account email
- **Account type**: Free, paid, or business account
- **Subscription information**: Current plan and features, storage used...


## Examples

### Basic user information

```bash
# View your account details
pass-cli user info
```

## Privacy and security

### Information sensitivity

The `user info` command shows:

- ✅ **Safe to display**: Email, plan type, feature availability
- 🔒 **Never shown**: Passwords, private keys, payment information

## Troubleshooting

### Cannot access user info

If the command fails:

1. **Authentication**: Ensure you're logged in with `pass-cli login`
2. **Network**: Check internet connectivity
3. **Account status**: Your account might be suspended or locked

## Related commands

- **[login](login.md)** - Authenticate to access user information
- **[info](info.md)** - Session-specific information (different from user info)
- **[vault](vault.md)** - Manage vaults owned by your user account
