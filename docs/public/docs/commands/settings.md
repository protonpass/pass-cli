# `settings` command

Manage persistent user settings for Proton Pass CLI.

## Synopsis

```bash
pass-cli settings <SUBCOMMAND>
```

## Description

The `settings` command allows you to configure persistent preferences for the CLI that apply across all commands. These settings are stored per user and persist between sessions, making it easier to work with your most frequently used vaults and preferred output formats.

## Subcommands

### view

View all current user settings.

```bash
pass-cli settings view
```

**Description:**

Displays all configured settings along with their current values. Settings that have not been set will show their default values.

**Examples:**

```bash
# View all settings
pass-cli settings view
```

### set

Set a user preference value.

```bash
pass-cli settings set <SETTING>
```

#### set default-vault

Set the default vault for item operations.

```bash
pass-cli settings set default-vault (--vault-name VAULT_NAME | --share-id SHARE_ID)
```

**Description:**

Configures a default vault that will be used automatically when you don't specify `--share-id` or `--vault-name` in item-related commands. This is particularly useful if you have one primary vault you work with frequently.

**Options:**

- `--vault-name VAULT_NAME` - Name of the vault to set as default
- `--share-id SHARE_ID` - Share ID of the vault to set as default

**Mutually exclusive options:**

- `--vault-name` and `--share-id` are mutually exclusive. You must provide exactly one.

**Affected commands:**

When a default vault is set, the following commands will use it automatically if no vault is specified:

- `item list` - List items from the default vault
- `item view` - View items from the default vault (when not using URI)
- `item totp` - Generate TOTP codes from items in the default vault (when not using URI)
- `item create` - Create items in the default vault
- `item move` - Use as source vault when `--from-share-id` / `--from-vault-name` are not specified
- `item trash` - Move items to trash in the default vault
- `item untrash` - Restore items in the default vault
- `item update` - Update items in the default vault

**Examples:**

```bash
# Set default vault by name
pass-cli settings set default-vault --vault-name "Personal Vault"

# Set default vault by share ID
pass-cli settings set default-vault --share-id "3GqM1RhVZL8uXR_abc123"

# After setting, you can omit vault parameters in commands:
pass-cli item list  # Lists items from your default vault
pass-cli item create login --title "New Login" --username "user"  # Creates in default vault
```

**Notes:**

- You cannot set an item share as the default vault
- The vault must exist and you must have access to it
- If the default vault is deleted or access is revoked, commands will fail until you set a new default or specify vaults explicitly

#### set default-format

Set the default output format for commands.

```bash
pass-cli settings set default-format <FORMAT>
```

**Description:**

Configures the default output format used by commands that support formatted output. This eliminates the need to specify `--output` on every command invocation.

**Arguments:**

- `FORMAT` - Output format: `human` or `json`

**Affected commands:**

When a default format is set, the following commands will use it automatically if `--output` is not specified:

- `item list` - List items in the specified format
- `item view` - Display item details in the specified format
- `item totp` - Show TOTP codes in the specified format

**Examples:**

```bash
# Set default output to human-readable format (default)
pass-cli settings set default-format human

# Set default output to JSON format
pass-cli settings set default-format json

# After setting, commands use your preferred format:
pass-cli item list  # Outputs in JSON if you set json as default
pass-cli item view "pass://MyVault/MyItem"  # Uses your default format

# You can still override on a per-command basis:
pass-cli item list --output human  # Forces human format regardless of default
```

### unset

Clear a user preference value, reverting it to the default.

```bash
pass-cli settings unset <SETTING>
```

#### unset default-vault

Clear the default vault setting.

```bash
pass-cli settings unset default-vault
```

**Description:**

Removes the configured default vault. After unsetting, commands will require explicit vault specification via `--share-id` or `--vault-name`.

**Examples:**

```bash
# Clear the default vault setting
pass-cli settings unset default-vault

# After unsetting, you must specify vaults explicitly:
pass-cli item list "Personal"
```

#### unset default-format

Clear the default output format setting.

```bash
pass-cli settings unset default-format
```

**Description:**

Removes the configured default output format. After unsetting, commands will use their built-in default (typically `human` format).

**Examples:**

```bash
# Clear the default format setting
pass-cli settings unset default-format

# After unsetting, commands default to human format:
pass-cli item list  # Uses human format
```

## Available settings

### `default-vault`

**Type:** Vault reference (Share ID)
**Default:** `(none)`
**Description:** The default vault used for item operations when not explicitly specified.

### `default-format`

**Type:** String (`human` or `json`)
**Default:** `human`
**Description:** The default output format for commands that support formatted output.

## Overriding defaults

All defaults can be overridden on a per-command basis by explicitly providing the relevant parameters:

```bash
# Even with a default vault set
pass-cli settings set default-vault --vault-name "Personal"

# You can still work with other vaults explicitly
pass-cli item list --vault-name "Work"
pass-cli item create login --share-id "abc123" --title "Item"

# Same for output format
pass-cli settings set default-format json
pass-cli item list --output human  # Override to human format
```
