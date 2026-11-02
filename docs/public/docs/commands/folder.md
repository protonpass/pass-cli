# `folder` command

Manage folders inside Proton Pass vaults.

## Synopsis

```bash
pass-cli folder <SUBCOMMAND>
```

## Description

The `folder` command provides operations for managing folders, which are optional groupings inside a vault. Folders can be nested, so you can organize items in a tree structure within a single vault instead of creating more vaults.

## How it works

Folders live inside a vault and belong to it. Each folder:

- Has a unique Folder ID within its vault
- Can contain items and other folders
- Can sit at the vault root or under a parent folder
- Inherits the permissions of the vault it belongs to

Every folder operation requires you to identify the vault, either by Share ID or by name. The CLI resolves the name to a Share ID internally. Folder IDs are returned by `folder create` and shown by `folder list`.

## Subcommands

### list

List folders in a vault.

```bash
pass-cli folder list (--share-id SHARE_ID | --vault-name NAME) [--parent-id FOLDER_ID] [--output FORMAT]
```

**Options:**

- `--share-id SHARE_ID` - Share ID of the vault
- `--vault-name NAME` - Name of the vault
- `--parent-id FOLDER_ID` - Only show folders whose parent is this folder ID
- `--output FORMAT` - Output format: `human` or `json`. Defaults to `human`

**Mutually exclusive options:**

- `--share-id` and `--vault-name` are mutually exclusive. You must provide exactly one.

**Examples:**

```bash
# List all folders in a vault by name
pass-cli folder list --vault-name "Personal Accounts"

# List all folders in a vault by Share ID, in JSON format
pass-cli folder list --share-id "abc123def" --output json

# List only the direct children of a folder
pass-cli folder list --vault-name "Personal Accounts" --parent-id "folder_id_here"
```

Human readable output shows the name, the ID, and the parent when the folder is nested:

```text
- Banking: pFqC6-0-2
- Proton: pFqC7-0-2 (parent: pFqC6-0-2)
```

### create

Create a new folder. Prints the ID of the new folder.

```bash
pass-cli folder create (--share-id SHARE_ID | --vault-name NAME) --name NAME [--parent-id FOLDER_ID]
```

**Options:**

- `--share-id SHARE_ID` - Share ID of the vault
- `--vault-name NAME` - Name of the vault
- `--name NAME` - Name of the folder (required)
- `--parent-id FOLDER_ID` - Parent folder ID, for a nested folder. Omit to create at the vault root

**Mutually exclusive options:**

- `--share-id` and `--vault-name` are mutually exclusive. You must provide exactly one.

**Examples:**

```bash
# Create a folder at the vault root
pass-cli folder create --vault-name "Personal Accounts" --name "Banking"

# Create a nested folder
pass-cli folder create --share-id "abc123def" --name "Credit Cards" --parent-id "folder_id_here"
```

### update

Rename an existing folder.

```bash
pass-cli folder update (--share-id SHARE_ID | --vault-name NAME) --folder-id FOLDER_ID --name NEW_NAME
```

**Options:**

- `--share-id SHARE_ID` - Share ID of the vault
- `--vault-name NAME` - Name of the vault
- `--folder-id FOLDER_ID` - ID of the folder to update (required)
- `--name NEW_NAME` - New name for the folder (required)

**Mutually exclusive options:**

- `--share-id` and `--vault-name` are mutually exclusive. You must provide exactly one.

**Examples:**

```bash
# Rename a folder
pass-cli folder update --share-id "abc123def" --folder-id "folder_id_here" --name "Retail Banking"
```

### delete

Delete a folder.

```bash
pass-cli folder delete (--share-id SHARE_ID | --vault-name NAME) --folder-id FOLDER_ID
```

**Options:**

- `--share-id SHARE_ID` - Share ID of the vault
- `--vault-name NAME` - Name of the vault
- `--folder-id FOLDER_ID` - ID of the folder to delete (required)

**Mutually exclusive options:**

- `--share-id` and `--vault-name` are mutually exclusive. You must provide exactly one.

**Examples:**

```bash
# Delete a folder by Share ID
pass-cli folder delete --share-id "abc123def" --folder-id "folder_id_here"

# Delete a folder by vault name
pass-cli folder delete --vault-name "Personal Accounts" --folder-id "folder_id_here"
```

### move

Move a folder to a different parent.

```bash
pass-cli folder move (--share-id SHARE_ID | --vault-name NAME) --folder-id FOLDER_ID [--parent-folder-id FOLDER_ID]
```

**Options:**

- `--share-id SHARE_ID` - Share ID of the vault
- `--vault-name NAME` - Name of the vault
- `--folder-id FOLDER_ID` - ID of the folder to move (required)
- `--parent-folder-id FOLDER_ID` - New parent folder ID. Omit to move the folder to the vault root

**Mutually exclusive options:**

- `--share-id` and `--vault-name` are mutually exclusive. You must provide exactly one.

**Examples:**

```bash
# Move a folder under another folder
pass-cli folder move --share-id "abc123def" --folder-id "child_folder_id" --parent-folder-id "new_parent_folder_id"

# Move a folder to the vault root
pass-cli folder move --share-id "abc123def" --folder-id "child_folder_id"
```

## Working with folder IDs

Folder IDs are unique identifiers within a vault. You can find them by:

1. Listing your folders: `pass-cli folder list --vault-name "My Vault"`
2. Looking for the ID in the output
3. Using the ID in other folder commands

Folder names are not guaranteed to be unique, so prefer IDs in scripts.

## Examples

### Complete folder workflow

```bash
# Create a folder and capture its ID
PARENT_ID=$(pass-cli folder create --vault-name "Team Project" --name "Production")

# Create a nested folder under it
CHILD_ID=$(pass-cli folder create --vault-name "Team Project" --name "Staging" --parent-id "$PARENT_ID")

# Inspect the result
pass-cli folder list --vault-name "Team Project" --output json

# Rename the nested folder
pass-cli folder update --vault-name "Team Project" --folder-id "$CHILD_ID" --name "QA"

# Move it to the vault root, then back under the parent
pass-cli folder move --vault-name "Team Project" --folder-id "$CHILD_ID"
pass-cli folder move --vault-name "Team Project" --folder-id "$CHILD_ID" --parent-folder-id "$PARENT_ID"
```

### Put items in a folder

Every `item create` subcommand accepts `--folder-id`, so items can be created straight into a folder.

```bash
# Get the folder ID first
FOLDER_ID=$(pass-cli folder list --vault-name "Team Project" --output json \
  | jq -r '.[] | select(.content.name == "Production") | .id')

# Create the item inside that folder
pass-cli item create login \
  --vault-name "Team Project" \
  --title "Admin Console" \
  --username "admin" \
  --generate-password \
  --folder-id "$FOLDER_ID"

# A note works the same way
pass-cli item create note \
  --vault-name "Team Project" \
  --title "Deployment runbook" \
  --note "Step by step release process" \
  --folder-id "$FOLDER_ID"
```

### Move existing items into a folder

Items that already exist are relocated with [`item move`](item.md#move). Nothing is recreated, and the item keeps its ID.

```bash
# Move an existing item into a folder of the same vault
pass-cli item move \
  --from-vault-name "Team Project" \
  --item-title "Admin Console" \
  --to-folder-id "$FOLDER_ID"

# Move it back to the vault root
pass-cli item move \
  --from-vault-name "Team Project" \
  --item-title "Admin Console" \
  --to-share-root
```

The source vault is named with `--from-vault-name` or `--from-share-id`, not with `--vault-name`. The destination folder must belong to that same vault.

### Find a folder ID in a script

```bash
#!/bin/bash
VAULT="Team Project"

# Extract the ID of the folder named "Production"
FOLDER_ID=$(pass-cli folder list --vault-name "$VAULT" --output json \
  | jq -r '.[] | select(.content.name == "Production") | .id')

if [ -z "$FOLDER_ID" ]; then
  echo "Folder not found, creating it"
  FOLDER_ID=$(pass-cli folder create --vault-name "$VAULT" --name "Production")
fi

echo "Production folder ID: $FOLDER_ID"
```

## Best practices

- Use folders for structure inside a vault, and vaults for permission boundaries, since folders inherit vault permissions
- Pass `--folder-id` on `item create` for new items, and use `item move --to-folder-id` for items that already exist
- Use `item move --to-share-root` to remove an item from its folder without deleting and recreating it
- Keep the hierarchy shallow. Deep trees are harder to navigate from the CLI
- Use IDs rather than names in scripts, because names can repeat within a vault
- Run `folder list` after structural changes to confirm the resulting layout

## Troubleshooting

### Folder not found

- Re-run `pass-cli folder list` against the same vault to confirm the ID
- Folder IDs are scoped to a vault. An ID from one vault is not valid in another
- Folder IDs are case-sensitive

### Wrong vault

- Operations fail if the vault cannot be resolved from `--vault-name`. Check the exact name with `pass-cli vault list`
- Use `--share-id` when several vaults have similar names

### Empty output from list

- A vault without folders produces no output in `human` format (or `[]` with `--output json`). This is not an error
- When `--parent-id` is set, only direct children of that folder are shown

## Related commands

- **[item](item.md)** - Manage items stored in vaults and folders
- **[vault](vault.md)** - Manage vaults that contain folders
