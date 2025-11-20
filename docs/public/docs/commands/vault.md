# `vault` command

Manage vaults in Proton Pass.

## Synopsis

```bash
pass-cli vault <SUBCOMMAND>
```

## Description

The `vault` command provides operations for managing vaults, which are containers that organize your items. You can create, list, update, delete, and share vaults, as well as manage vault members.

## How it works

Vaults are the top-level organizational structure in Proton Pass. Each vault:

- Contains multiple items (logins, notes, passwords, etc.)
- Has a unique Share ID for identification
- Can be shared with other users with different permission levels
- Can have multiple members with different roles

Most vault operations allow you to reference a vault either by its Share ID or by its name. The CLI will resolve the name to a Share ID internally.

## Subcommands

### list

List all vaults you have access to.

```bash
pass-cli vault list [--output FORMAT]
```

**Options:**

- `--output FORMAT` - Output format: `human` (default) or `json`

**Examples:**

```bash
# List vaults in human-readable format
pass-cli vault list

# List vaults in JSON format for scripting
pass-cli vault list --output json
```

### create

Create a new vault.

```bash
pass-cli vault create --name NAME
```

**Options:**

- `--name NAME` - Name of the vault (required)

**Examples:**

```bash
# Create a personal vault
pass-cli vault create --name "Personal Accounts"

# Create a work vault
pass-cli vault create --name "Work Projects"
```

### update

Update an existing vault's properties (currently only the name).

```bash
pass-cli vault update (--share-id SHARE_ID | --vault-name NAME) --name NEW_NAME
```

**Options:**

- `--share-id SHARE_ID` - Share ID of the vault to update
- `--vault-name NAME` - Name of the vault to update
- `--name NEW_NAME` - New name for the vault (required)

**Mutually exclusive options:**

- `--share-id` and `--vault-name` are mutually exclusive. You must provide exactly one.

**Examples:**

```bash
# Rename a vault by Share ID
pass-cli vault update --share-id "abc123def" --name "Updated Vault Name"

# Rename a vault by name
pass-cli vault update --vault-name "Old Name" --name "New Name"
```

### delete

Delete a vault and all its contents.

```bash
pass-cli vault delete (--share-id SHARE_ID | --vault-name NAME)
```

**Options:**

- `--share-id SHARE_ID` - Share ID of the vault to delete
- `--vault-name NAME` - Name of the vault to delete

**Mutually exclusive options:**

- `--share-id` and `--vault-name` are mutually exclusive. You must provide exactly one.

**Examples:**

```bash
# Delete a vault by Share ID (this also deletes all items in it!)
pass-cli vault delete --share-id "abc123def"

# Delete a vault by name
pass-cli vault delete --vault-name "Old Vault"
```

**⚠️ Warning:** This permanently deletes the vault and all items within it. This action cannot be undone.

### member

Manage vault members. This is a subcommand with its own operations.

```bash
pass-cli vault member <SUBCOMMAND>
```

**Subcommands:**

- `list` - List members of a vault
- `update` - Update a member's role
- `remove` - Remove a member from a vault

**Examples:**

```bash
# List vault members (by Share ID)
pass-cli vault member list --share-id "abc123def"

# List vault members (by name)
pass-cli vault member list --vault-name "Team Vault"

# List vault members (with json output)
pass-cli vault member list --vault-name "Team Vault" --output=json

# Update member role
pass-cli vault member update --share-id "abc123def" --member-share-id "member123" --role editor

# Remove a member
pass-cli vault member remove --share-id "abc123def" --member-share-id "member123"
```

**Note:** All member subcommands support both `--share-id` and `--vault-name` options (mutually exclusive).

### share

Share a vault with another user.

```bash
pass-cli vault share (--share-id SHARE_ID | --vault-name NAME) EMAIL [--role ROLE]
```

**Options:**

- `--share-id SHARE_ID` - Share ID of the vault to share
- `--vault-name NAME` - Name of the vault to share
- `EMAIL` - Email address of the user to share with (required)
- `--role ROLE` - Role to assign: `viewer`, `editor`, or `manager` (default: `viewer`)

**Mutually exclusive options:**

- `--share-id` and `--vault-name` are mutually exclusive. You must provide exactly one.

**Examples:**

```bash
# Share vault with viewer access (by Share ID)
pass-cli vault share --share-id "abc123def" colleague@company.com

# Share vault with editor access (by name)
pass-cli vault share --vault-name "Team Vault" colleague@company.com --role editor

# Share vault with manager access
pass-cli vault share --share-id "abc123def" colleague@company.com --role manager
```

## Share roles

When sharing vaults, you can assign different roles:

- **viewer** - Can view the vault and all items within it
- **editor** - Can view and modify items, create new items
- **manager** - Full access including sharing with others and managing members

## Working with share IDs

Share IDs are unique identifiers for vaults. You can find them by:

1. Listing your vaults: `pass-cli vault list`
2. Looking for the share ID in the output
3. Using the share ID in other commands

## Examples

### Complete vault workflow

```bash
# Create a new vault
pass-cli vault create --name "Team Project"

# List vaults to get the share ID
pass-cli vault list

# Share the vault with team members
pass-cli vault share --share-id "new_vault_id" alice@company.com --role editor
pass-cli vault share --share-id "new_vault_id" bob@company.com --role viewer

# Check who has access
pass-cli vault members --share-id "new_vault_id"
```

### Vault management script

```bash
#!/bin/bash
VAULT_NAME="Project Alpha"
SHARE_ID="abc123def"

# Create vault
echo "Creating vault: $VAULT_NAME"
pass-cli vault create --name "$VAULT_NAME"

# Share with team
echo "Sharing with team members"
pass-cli vault share --share-id "$SHARE_ID" alice@company.com --role editor
pass-cli vault share --share-id "$SHARE_ID" bob@company.com --role editor

# List final members
echo "Vault members:"
pass-cli vault members --share-id "$SHARE_ID"
```

## Best practices

### Naming conventions

- Use descriptive names that indicate the vault's purpose
- Consider prefixes for organization (e.g., "Work - ", "Personal - ")
- Keep names concise but meaningful

### Sharing strategy

- Start with minimal permissions (viewer) and increase as needed
- Regularly review vault members and their roles
- Use manager role sparingly - only for trusted administrators

### Organization

- Create separate vaults for different contexts (work, personal, projects)
- Group related items together in the same vault
- Consider vault sharing boundaries when organizing

## Troubleshooting

### Permission errors

- Ensure you have manager rights to perform administrative operations
- Verify you're using the correct share ID
- Check that the target user has a Proton Pass account

### Share ID issues

- Double-check the share ID from `pass-cli vault list`
- Share IDs are case-sensitive
- Ensure you're copying the complete ID

### transfer

Transfer ownership of a vault to another member.

```bash
pass-cli vault transfer (--share-id SHARE_ID | --vault-name NAME) MEMBER_SHARE_ID
```

**Options:**

- `--share-id SHARE_ID` - Share ID of the vault to transfer
- `--vault-name NAME` - Name of the vault to transfer
- `MEMBER_SHARE_ID` - Share ID of the member who will become the new owner (required)

**Mutually exclusive options:**

- `--share-id` and `--vault-name` are mutually exclusive. You must provide exactly one.

**Examples:**

```bash
# Transfer vault ownership by Share ID
pass-cli vault transfer --share-id "abc123def" "member_share_id_xyz"

# Transfer vault ownership by name
pass-cli vault transfer --vault-name "My Vault" "member_share_id_xyz"
```

## Related commands

- **[item](item.md)** - Manage items within vaults
- **[share](share.md)** - Manage all types of shares
- **[user](user.md)** - User account information
