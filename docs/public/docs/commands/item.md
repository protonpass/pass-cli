# `item` command

Manage items in Proton Pass vaults.

## Synopsis

```bash
pass-cli item <SUBCOMMAND>
```

## Description

The `item` command provides operations for managing items within vaults. Items are the fundamental units of data storage in Proton Pass, including logins, notes, credit cards, and aliases.

## Subcommands

### list

List items in vaults.

```bash
pass-cli item list [VAULT_NAME] [--share-id SHARE_ID] [--output FORMAT]
```

**Options:**

- `VAULT_NAME` - Name of the vault to list items from (optional)
- `--share-id SHARE_ID` - Share ID of the vault to list items from (optional)
- `--output FORMAT` - Output format: `human` (default) or `json`

**Examples:**

```bash
# List all items in all accessible vaults
pass-cli item list

# List items in a specific vault by name
pass-cli item list "Personal Vault"

# List items in a specific vault by share ID
pass-cli item list --share-id "abc123def"

# List items in JSON format
pass-cli item list --output json
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

- `--share-id SHARE_ID` - Share ID of the vault to create the item in (required)
- `--title TITLE` - Title of the login item (required unless using template)
- `--username USERNAME` - Username for the login (optional)
- `--email EMAIL` - Email for the login (optional)  
- `--password PASSWORD` - Password for the login (optional)
- `--generate-password[=SETTINGS]` - Generate a random password (optional)
- `--generate-passphrase[=WORD_COUNT]` - Generate a passphrase (optional)
- `--url URL` - Associated URLs (can be used multiple times)
- `--get-template` - Output a JSON template structure
- `--from-template FILE` - Create from template file or `-` for stdin

**Examples:**

```bash
# Create a basic login item
pass-cli item create login \
  --share-id "abc123def" \
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
  --share-id "abc123def" \
  --title "Secure Account" \
  --username "myuser" \
  --generate-password="20,true,true" \
  --url "https://example.com"

# Get login template structure
pass-cli item create login --get-template > template.json

# Create from template file
pass-cli item create login --from-template template.json --share-id "abc123def"

# Create from stdin template
echo '{"title":"Test Login","username":"user","password":"pass","urls":["https://test.com"]}' | \
  pass-cli item create login --share-id "abc123def" --from-template -
```

### view

View an item's details.

```bash
pass-cli item view [OPTIONS] [URI]
```

**Options:**

- `--share-id SHARE_ID` - Share ID of the vault containing the item
- `--vault-name VAULT_NAME` - Name of the vault containing the item
- `--item-id ITEM_ID` - ID of the item
- `--item-title ITEM_TITLE` - Title of the item
- `URI` - Pass URI in format `pass://SHARE_ID/ITEM_ID[/FIELD]`
- `--field FIELD` - Specific field to view
- `--output FORMAT` - Output format: `human` (default) or `json`

**Mutually exclusive options:**

- `--share-id` and `--vault-name` are mutually exclusive. You must provide exactly one.
- `--item-id` and `--item-title` are mutually exclusive. You must provide exactly one.
- - `--share-id/--vault-name` and `--item-id/--item-title` parameters and `URI` are mutually exclusive. You must provide either both parameters or a single secret reference.

**Examples:**

```bash
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

- `--share-id SHARE_ID` - Share ID of the vault containing the item
- `--vault-name VAULT_NAME` - Name of the vault containing the item
- `--item-id ITEM_ID` - ID of the item to update
- `--item-title ITEM_TITLE` - Title of the item to update
- `--field FIELD_NAME=FIELD_VALUE` - Field to update in format `field_name=field_value`. Can be specified multiple times to update multiple fields.

**Mutually exclusive options:**

- `--share-id` and `--vault-name` are mutually exclusive. You must provide exactly one.
- `--item-id` and `--item-title` are mutually exclusive. You must provide exactly one.
- `--share-id/--vault-name` and `--item-id/--item-title` parameters and `URI` are mutually exclusive. You must provide either both parameters or a single secret reference.
- At least one `--field` option is required.

**Field names:**

Standard fields for login items include: `title`, `username`, `password`, `email`, `url`, `note`. You can also create or update custom fields with any name.

!!! info "Types of fields"

    Item update does not allow to change time or TOTP fields. Please use a different Proton Pass client to update those fields

**Examples:**

### Update a single field

```bash
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

**Examples:**

```bash
# Alias operations (exact subcommands depend on implementation)
pass-cli item alias create --share-id "abc123def" --title "Shopping Alias"
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

- `--share-id SHARE_ID` - Share ID of the vault containing the item
- `--vault-name VAULT_NAME` - Name of the vault containing the item
- `--item-id ITEM_ID` - ID of the item
- `--item-title ITEM_TITLE` - Title of the item
- `URI` - Pass URI in format `pass://SHARE_ID/ITEM_ID[/FIELD]`
- `--field FIELD` - Specific field to view
- `--output FORMAT` - Output format: `human` (default) or `json`

**Mutually exclusive options:**

- `--share-id` and `--vault-name` are mutually exclusive. You must provide exactly one.
- `--item-id` and `--item-title` are mutually exclusive. You must provide exactly one.
- `--share-id/--vault-name` and `--item-id/--item-title` parameters and `URI` are mutually exclusive. You must provide either both parameters or a single secret reference.

**Examples:**

```bash
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
