# `item` command

Manage items in Proton Pass vaults.

## Synopsis

```bash
pass-cli item <SUBCOMMAND>
```

## Description

The `item` command provides operations for managing items within vaults. Items are the fundamental units of data storage in Proton Pass, including logins, notes, credit cards, and aliases.

!!! tip "Using default settings"

    You can configure a default vault and output format using the [`settings`](settings.md) command. When defaults are set, many item commands no longer require `--share-id`, `--vault-name`, or `--output` parameters, making your workflow more efficient.

## Subcommands

### list

List items in vaults.

```bash
pass-cli item list [VAULT_NAME] [--share-id SHARE_ID] [--output FORMAT]
```

**Options:**

- `VAULT_NAME` - Name of the vault to list items from. Specify `VAULT_NAME` if you are not passing a `--share-id`. Used as an argument
- `--share-id SHARE_ID` - Share ID of the vault to list items from. Specify `--share-id` if you are not passing a `VAULT_NAME`. Used as a flag
- `--output FORMAT` - Output format: `human` or `json`. Uses default format from settings if not specified.

**Mutually exclusive options:**

- `--share-id` and `VAULT_NAME` are mutually exclusive. You can provide one, or neither if a default vault is configured.

**Using default settings:**

If you have set a default vault using [`settings set default-vault`](settings.md#set-default-vault), you can omit both `VAULT_NAME` and `--share-id`. Similarly, if you've set a default output format, you can omit `--output`.

**Examples:**

```bash
# List items using default vault and format (requires settings configured)
pass-cli item list

# List items in a specific vault by name
pass-cli item list "Personal Vault"

# List items in a specific vault by share ID
pass-cli item list --share-id "abc123def"

# List items in JSON format
pass-cli item list --output json

# Using default vault but override format
pass-cli item list --output human
```

### create

Create new items in a vault.

```bash
pass-cli item create <ITEM_TYPE> [OPTIONS]
```

#### create login

Create a new login item.

```bash
pass-cli item create login [OPTIONS]
```

**Options:**

- `--share-id SHARE_ID` - Share ID of the vault to create the item in
- `--vault-name VAULT_NAME` - Name of the vault to create the item in
- `--title TITLE` - Title of the login item (required unless using template)
- `--username USERNAME` - Username for the login (optional)
- `--email EMAIL` - Email for the login (optional)
- `--password PASSWORD` - Password for the login (optional)
- `--generate-password[=SETTINGS]` - Generate a random password (optional)
- `--generate-passphrase[=WORD_COUNT]` - Generate a passphrase (optional)
- `--url URL` - Associated URLs (can be used multiple times)
- `--get-template` - Output a JSON template structure
- `--from-template FILE` - Create from template file or `-` for stdin

**Mutually exclusive options:**

- `--share-id` and `--vault-name` are mutually exclusive. You can provide one, or neither if a default vault is configured.

**Examples:**

```bash
# Create using default vault (if configured)
pass-cli item create login \
  --title "GitHub Account" \
  --username "myuser" \
  --password "mypassword" \
  --url "https://github.com"

# Create a basic login item using share ID
pass-cli item create login \
  --share-id "abc123def" \
  --title "GitHub Account" \
  --username "myuser" \
  --password "mypassword" \
  --url "https://github.com"

# Create a basic login item using vault name
pass-cli item create login \
  --vault-name "Personal" \
  --title "GitHub Account" \
  --username "myuser" \
  --password "mypassword" \
  --url "https://github.com"

# Create login with generated password
pass-cli item create login \
  --share-id "abc123def" \
  --title "New Account" \
  --username "myuser" \
  --generate-password \
  --url "https://example.com"

# Create login with custom password generation
pass-cli item create login \
  --vault-name "Work" \
  --title "Secure Account" \
  --username "myuser" \
  --generate-password="20,uppercase,symbols" \
  --url "https://example.com"

# Get login template structure
pass-cli item create login --get-template > template.json

# Create from template file using share ID
pass-cli item create login --from-template template.json --share-id "abc123def"

# Create from template file using vault name
pass-cli item create login --from-template template.json --vault-name "Personal"

# Create from stdin template
echo '{"title":"Test Login","username":"user","password":"pass","urls":["https://test.com"]}' | \
  pass-cli item create login --share-id "abc123def" --from-template -
```

#### create ssh-key

Create or import SSH key items. SSH keys can be either generated from scratch or imported from existing private key files.

```bash
pass-cli item create ssh-key <SUBCOMMAND>
```

**Subcommands:**

- `generate` - Generate a new SSH key pair
- `import` - Import an existing SSH key from a private key file

##### create ssh-key generate

Generate a new SSH key pair and store it in Proton Pass.

```bash
pass-cli item create ssh-key generate [OPTIONS]
```

**Options:**

- `--share-id SHARE_ID` - Share ID of the vault to create the SSH key in
- `--vault-name VAULT_NAME` - Name of the vault to create the SSH key in
- `--title TITLE` - Title of the SSH key item (required)
- `--key-type TYPE` - Type of SSH key to generate: `ed25519` (default), `rsa2048`, or `rsa4096`
- `--comment COMMENT` - Comment for the SSH key (optional)
- `--password` - Enable passphrase protection for the SSH key (optional)

**Mutually exclusive options:**

- `--share-id` and `--vault-name` are mutually exclusive. You can provide one, or neither if a default vault is configured.

**Passphrase protection:**

When generating new SSH keys with `--password`, you'll be prompted to enter and confirm a passphrase. The passphrase can also be provided via environment variables:

- `PROTON_PASS_SSH_KEY_PASSWORD` - Passphrase as plain text
- `PROTON_PASS_SSH_KEY_PASSWORD_FILE` - Path to file containing the passphrase

> [!NOTE]
> **Passphrase recommendation for generated keys**
> Since generated SSH keys are already encrypted and securely stored within your Proton Pass vault, adding a passphrase is optional. However, if you plan to export the key for use outside Proton Pass, adding a passphrase provides an additional layer of security.

**Examples:**

```bash
# Generate using default vault (if configured)
pass-cli item create ssh-key generate \
  --title "GitHub Deploy Key"

# Generate an Ed25519 key (recommended)
pass-cli item create ssh-key generate \
  --share-id "abc123def" \
  --title "GitHub Deploy Key"

# Generate an Ed25519 key using vault name
pass-cli item create ssh-key generate \
  --vault-name "Development Keys" \
  --title "GitHub Deploy Key"

# Generate an RSA 4096 key with comment
pass-cli item create ssh-key generate \
  --share-id "abc123def" \
  --title "Production Server" \
  --key-type rsa4096 \
  --comment "prod-server-deploy"

# Generate a passphrase-protected key
pass-cli item create ssh-key generate \
  --share-id "abc123def" \
  --title "Secure Key" \
  --password

# Generate with passphrase from environment variable
PROTON_PASS_SSH_KEY_PASSWORD="my-passphrase" \
  pass-cli item create ssh-key generate \
  --share-id "abc123def" \
  --title "Automated Key" \
  --password
```

##### create ssh-key import

Import an existing SSH private key from a file into Proton Pass.

```bash
pass-cli item create ssh-key import [OPTIONS]
```

**Options:**

- `--from-private-key PATH` - Path to the private key file (required)
- `--share-id SHARE_ID` - Share ID of the vault to create the SSH key in
- `--vault-name VAULT_NAME` - Name of the vault to create the SSH key in
- `--title TITLE` - Title of the SSH key item (required)
- `--password` - Prompt for the passphrase if the private key is encrypted (optional)

**Mutually exclusive options:**

- `--share-id` and `--vault-name` are mutually exclusive. You can provide one, or neither if a default vault is configured.

**Handling passphrase-protected SSH keys:**

If you're importing an existing SSH key that's protected with a passphrase, you have several options:

1. **Store the key with its passphrase protection intact:**
   - Use the `--password` flag when importing
   - The passphrase will be prompted interactively or read from environment variables
   - The key will be stored in Pass with its passphrase, and you'll need to provide it when using the key

2. **Remove the passphrase before importing (recommended):**
   - Create an unencrypted copy of your key: `ssh-keygen -p -f /path/to/key -N ""`
   - Import the unencrypted key into Proton Pass
   - Since the key will be encrypted within your vault, an additional passphrase is unnecessary
   - Delete the unencrypted copy after importing

3. **Store the passphrase in the same item:**
   - Import the passphrase-protected key
   - After import, use `pass-cli item update` to add the passphrase as a custom hidden field
   - The CLI will automatically try to use this field when loading the key into an SSH agent

**Passphrase environment variables:**

Similar to key generation, you can provide passphrases via:

- `PROTON_PASS_SSH_KEY_PASSWORD` - Passphrase as plain text
- `PROTON_PASS_SSH_KEY_PASSWORD_FILE` - Path to file containing the passphrase

**Examples:**

```bash
# Import an unencrypted SSH key
pass-cli item create ssh-key import \
  --from-private-key ~/.ssh/id_ed25519 \
  --share-id "abc123def" \
  --title "My SSH Key"

# Import using vault name
pass-cli item create ssh-key import \
  --from-private-key ~/.ssh/id_rsa \
  --vault-name "Personal Keys" \
  --title "Old RSA Key"

# Import a passphrase-protected key (will prompt for passphrase)
pass-cli item create ssh-key import \
  --from-private-key ~/.ssh/id_ed25519 \
  --share-id "abc123def" \
  --title "Protected Key" \
  --password

# Import with passphrase from environment variable
PROTON_PASS_SSH_KEY_PASSWORD="my-key-passphrase" \
  pass-cli item create ssh-key import \
  --from-private-key ~/.ssh/id_ed25519 \
  --share-id "abc123def" \
  --title "Automated Import" \
  --password

# Import with passphrase from file
PROTON_PASS_SSH_KEY_PASSWORD_FILE="/secure/passphrase.txt" \
  pass-cli item create ssh-key import \
  --from-private-key ~/.ssh/id_ed25519 \
  --share-id "abc123def" \
  --title "Key with File Passphrase" \
  --password
```

**Workflow for removing passphrase from existing key:**

```bash
# Step 1: Create a copy of your key without passphrase
cp ~/.ssh/id_ed25519 /tmp/id_ed25519_temp
ssh-keygen -p -f /tmp/id_ed25519_temp -N ""
# (You'll be prompted for the current passphrase)

# Step 2: Import the unencrypted copy
pass-cli item create ssh-key import \
  --from-private-key /tmp/id_ed25519_temp \
  --share-id "abc123def" \
  --title "My SSH Key"

# Step 3: Securely delete the temporary unencrypted copy
shred -u /tmp/id_ed25519_temp  # Linux
# or
rm -P /tmp/id_ed25519_temp  # macOS
```

> [!TIP]
> **Using imported SSH keys**
> Once imported, your SSH keys can be loaded into any SSH agent using the [`ssh-agent load`](./ssh-agent.md#ssh-agent-integration) command or by starting Proton Pass CLI's built-in SSH agent with [`ssh-agent start`](./ssh-agent.md#proton-pass-cli-as-your-ssh-agent).

### view

View an item's details.

```bash
pass-cli item view [OPTIONS] [URI]
```

**Options:**

- `--share-id SHARE_ID` - Share ID of the vault containing the item (optional if default vault is set and not using URI)
- `--vault-name VAULT_NAME` - Name of the vault containing the item (optional if default vault is set and not using URI)
- `--item-id ITEM_ID` - ID of the item
- `--item-title ITEM_TITLE` - Title of the item
- `URI` - Pass URI in format `pass://SHARE_ID/ITEM_ID[/FIELD]`
- `--field FIELD` - Specific field to view
- `--output FORMAT` - Output format: `human` or `json`. Uses default format from settings if not specified.

**Mutually exclusive options:**

- `--share-id` and `--vault-name` are mutually exclusive. You can provide one, or neither if a default vault is configured and not using URI.
- `--item-id` and `--item-title` are mutually exclusive. You must provide exactly one.
- `--share-id/--vault-name` and `--item-id/--item-title` parameters and `URI` are mutually exclusive. You must provide either both parameters or a single secret reference.

**Examples:**

```bash
# View item using default vault (if configured)
pass-cli item view --item-id "item456"

# View item by IDs
pass-cli item view --share-id "abc123def" --item-id "item456"

# View item by names
pass-cli item view --vault-name "MyVault" --item-title "MyItem"

# View item using Pass URI
pass-cli item view "pass://abc123def/item456"

# View item using Pass URI with names
pass-cli item view "pass://MyVault/MyItem"

# View specific field using URI
pass-cli item view "pass://abc123def/item456/password"

# View specific field using options
pass-cli item view --share-id "abc123def" --item-id "item456" --field "username"

# View item in JSON format
pass-cli item view --share-id "abc123def" --item-id "item456" --output json
```

### update

Update an item's fields.

```bash
pass-cli item update (--share-id SHARE_ID | --vault-name VAULT_NAME) (--item-id ITEM_ID | --item-title ITEM_TITLE) --field FIELD_NAME=FIELD_VALUE [--field FIELD_NAME=FIELD_VALUE]...
```

**Description:**

The `update` command allows you to modify fields of an existing item. You can update standard fields (like `title`, `username`, `password`, `email`, `url`) or create/update custom fields. Multiple fields can be updated in a single command.

**Options:**

- `--share-id SHARE_ID` - Share ID of the vault containing the item (optional if default vault is set)
- `--vault-name VAULT_NAME` - Name of the vault containing the item (optional if default vault is set)
- `--item-id ITEM_ID` - ID of the item to update
- `--item-title ITEM_TITLE` - Title of the item to update
- `--field FIELD_NAME=FIELD_VALUE` - Field to update in format `field_name=field_value`. Can be specified multiple times to update multiple fields.

**Mutually exclusive options:**

- `--share-id` and `--vault-name` are mutually exclusive. You can provide one, or neither if a default vault is configured.
- `--item-id` and `--item-title` are mutually exclusive. You must provide exactly one.
- At least one `--field` option is required.

**Field names:**

Standard fields for login items include: `title`, `username`, `password`, `email`, `url`, `note`. You can also create or update custom fields with any name.

> [!NOTE]
> **Types of fields**
> Item update does not allow to change time or TOTP fields. Please use a different Proton Pass client to update those fields

**Examples:**

### Update a single field

```bash
# Update password using default vault (if configured)
pass-cli item update \
  --item-id "item456" \
  --field "password=newpassword123"

# Update password by Share ID and Item ID
pass-cli item update \
  --share-id "abc123def" \
  --item-id "item456" \
  --field "password=newpassword123"

# Update password by vault name and item title
pass-cli item update \
  --vault-name "Personal" \
  --item-title "GitHub Account" \
  --field "password=newpassword123"
```

### Update multiple fields

```bash
# Update multiple fields at once
pass-cli item update \
  --share-id "abc123def" \
  --item-id "item456" \
  --field "username=newusername" \
  --field "password=newpassword" \
  --field "email=newemail@example.com"
```

### Update title

```bash
# Rename an item
pass-cli item update \
  --vault-name "Work" \
  --item-title "Old Title" \
  --field "title=New Title"
```

### Create or update custom fields

```bash
# Create/update custom fields
pass-cli item update \
  --share-id "abc123def" \
  --item-id "item456" \
  --field "api_key=sk_live_abc123" \
  --field "environment=production" \
  --field "notes=Updated on 2024-01-15"
```

### Update URL

```bash
# Update URL field
pass-cli item update \
  --share-id "abc123def" \
  --item-id "item456" \
  --field "url=https://newurl.com"
```

**Field value format:**

Field values are specified using the `field_name=field_value` format:

- Simple values: `--field "password=mypassword"`
- Values with spaces: `--field "title=My Account Title"`
- Values with special characters: `--field "url=https://example.com/path?query=value"`
- Values with equals signs: The first `=` separates the field name from the value

**Output:**

The command provides feedback on each field update:

```text
Updated field: password
Updated field: username
Created new custom field: api_key
Item updated successfully: 2 field(s) updated, 1 custom field(s) created
```

### delete

Delete an item.

```bash
pass-cli item delete --share-id SHARE_ID --item-id ITEM_ID
```

**Options:**

- `--share-id SHARE_ID` - Share ID of the vault containing the item (required)
- `--item-id ITEM_ID` - ID of the item to delete (required)

**Examples:**

```bash
# Delete an item
pass-cli item delete --share-id "abc123def" --item-id "item456"
```

!!! danger "Permanent deletion!"

    This permanently deletes the item. This action cannot be undone.

### share

Share an item with another user.

```bash
pass-cli item share --share-id SHARE_ID --item-id ITEM_ID EMAIL [--role ROLE]
```

**Options:**

- `--share-id SHARE_ID` - Share ID of the vault containing the item (required)
- `--item-id ITEM_ID` - ID of the item to share (required)
- `EMAIL` - Email address of the user to share with (required)
- `--role ROLE` - Role to assign: `viewer`, `editor`, or `manager` (default: `viewer`)

**Examples:**

```bash
# Share item with viewer access
pass-cli item share --share-id "abc123def" --item-id "item456" colleague@company.com

# Share item with editor access  
pass-cli item share --share-id "abc123def" --item-id "item456" colleague@company.com --role editor
```

### attachment

Manage item attachments.

```bash
pass-cli item attachment <ATTACHMENT_SUBCOMMAND>
```

#### attachment download

Download an attachment from an item.

```bash
pass-cli item attachment download [OPTIONS]
```

**Examples:**

```bash
# Download attachment (exact options depend on implementation)
pass-cli item attachment download --share-id "abc123def" --item-id "item456" --attachment-id "att789"
```

### alias

Manage email aliases.

```bash
pass-cli item alias <ALIAS_SUBCOMMAND>
```

#### alias create

Create a new email alias.

```bash
pass-cli item alias create [OPTIONS]
```

**Options:**

- `--share-id SHARE_ID` - Share ID of the vault where the alias will be created
- `--vault-name VAULT_NAME` - Name of the vault where the alias will be created
- `--prefix PREFIX` - Prefix of the alias. The resulting email will be `[prefix].[suffix]` (required)
- `--output FORMAT` - Output format: `human` or `json`. Uses default format from settings if not specified.

**Mutually exclusive options:**

- `--share-id` and `--vault-name` are mutually exclusive. You can provide one, or neither if a default vault is configured.

**Examples:**

```bash
# Create alias using default vault (if configured)
pass-cli item alias create --prefix "shopping"

# Create alias using share ID
pass-cli item alias create --share-id "abc123def" --prefix "newsletter"

# Create alias using vault name
pass-cli item alias create --vault-name "Personal" --prefix "work-signup"

# Create alias with JSON output
pass-cli item alias create --vault-name "Personal" --prefix "temp" --output json
```

## Login template format

When using `--get-template` or `--from-template`, the JSON structure is:

```json
{
  "title": "Item Title",
  "username": "optional_username",
  "email": "optional_email@example.com",
  "password": "optional_password",
  "urls": ["https://example.com", "https://app.example.com"]
}
```

### totp

Generate TOTP codes for the fields of an item. If you have an item that has TOTP fields associated to it, you can get the values of the TOTPs by using `item totp`.

**Options:**

- `--share-id SHARE_ID` - Share ID of the vault containing the item (optional if default vault is set and not using URI)
- `--vault-name VAULT_NAME` - Name of the vault containing the item (optional if default vault is set and not using URI)
- `--item-id ITEM_ID` - ID of the item
- `--item-title ITEM_TITLE` - Title of the item
- `URI` - Pass URI in format `pass://SHARE_ID/ITEM_ID[/FIELD]`
- `--field FIELD` - Specific field to view
- `--output FORMAT` - Output format: `human` or `json`. Uses default format from settings if not specified.

**Mutually exclusive options:**

- `--share-id` and `--vault-name` are mutually exclusive. You can provide one, or neither if a default vault is configured and not using URI.
- `--item-id` and `--item-title` are mutually exclusive. You must provide exactly one.
- `--share-id/--vault-name` and `--item-id/--item-title` parameters and `URI` are mutually exclusive. You must provide either both parameters or a single secret reference.

**Examples:**

```bash
# Generate TOTP using default vault and format (if configured)
pass-cli item totp --item-title "WithTOTPs"

# Generate all TOTPs on a given item (Readable format)
pass-cli item totp "pass://TOTP export/WithTOTPs"
# TOTP 1: 343325
# TOTP 2: 068223
# totp: 639378

# Generate all TOTPs on a given item (JSON format)
pass-cli item totp "pass://TOTP export/WithTOTPs" --output=json
# {
#   "TOTP 2": "622653",
#   "TOTP 1": "119533",
#   "totp": "152470",
# }

# Generate the TOTP for a given item and field
pass-cli item totp "pass://TOTP export/WithTOTPs/TOTP 1"
# TOTP 1: 343325

# Generate the TOTP for a given item and field and get only the value
pass-cli item totp "pass://TOTP export/WithTOTPs/TOTP 1" --output=json | jq -r '."TOTP 1"'
# 474540
```

## Password generation

### Random passwords

Use `--generate-password` with optional settings:

- Format: `"length,uppercase,symbols"`
- Example: `--generate-password="16,true,true"` (16 chars, with uppercase and symbols)
- Default: `--generate-password` (uses default settings)

### Passphrases

Use `--generate-passphrase` with optional word count:

- Format: `"word_count"`
- Example: `--generate-passphrase="5"` (5-word passphrase)
- Default: `--generate-passphrase` (uses default word count)

## Examples

### Complete item workflow

```bash
# Create a vault first
pass-cli vault create --name "Web Accounts"
SHARE_ID="new_vault_share_id"  # Get this from vault list

# Create a login item
pass-cli item create login \
  --share-id "$SHARE_ID" \
  --title "GitHub Account" \
  --username "developer123" \
  --generate-password \
  --url "https://github.com" \
  --url "https://github.com/login"

# List items to find the new item
pass-cli item list --share-id "$SHARE_ID"

# View the created item
ITEM_ID="new_item_id"  # Get this from item list
pass-cli item view --share-id "$SHARE_ID" --item-id "$ITEM_ID"

# Get just the password
pass-cli item view "pass://$SHARE_ID/$ITEM_ID/password"
```

## Best practices

### Item organization

- Use descriptive titles that make items easy to find
- Include relevant URLs for login items
- Group related items in the same vault

### Security

- Use generated passwords when possible
- Regularly update passwords for important accounts
- Be cautious when sharing individual items

### Templates

- Use templates for consistent item creation
- Validate template JSON before using them

## Troubleshooting

### Creation issues

- Verify you have edit permissions for the target vault
- Check that required fields are provided
- Ensure JSON templates are valid

### Access issues  

- Confirm you have access to the vault containing the item
- Verify share IDs and item IDs are correct
- Check that items haven't been deleted

## Related commands

- **[vault](vault.md)** - Manage vaults that contain items
- **[password](password.md)** - Generate and analyze passwords
- **[share](share.md)** - Manage item and vault shares
