# Folder object

A **Folder** is an optional grouping inside a vault. Folders let you organize items into a hierarchy without creating additional vaults. An item either lives at the vault root or inside a single folder.

``` mermaid
flowchart TD
    v1[("Vault")]
    f1["Folder: Banking"]
    f2["Folder: Investments"]
    f3["Folder: Proton"]
    i1@{ shape: notch-rect, label: "Login item at vault root"}
    i2@{ shape: notch-rect, label: "Login item in Banking"}
    i3@{ shape: notch-rect, label: "Note item in Proton"}
    v1 --> i1
    v1 --> f1
    v1 --> f2
    f1 --> f3
    f1 --> i2
    f3 --> i3
```

## How folders work

- Folders belong to a single vault. They are identified by a Folder ID, scoped to the vault's Share ID.
- Folders can be nested. A folder can contain items and other folders, so you can build a tree of arbitrary depth.
- Items are not required to be in a folder. Items without a folder live at the vault root.
- Folder names do not have to be unique within a vault. Use the Folder ID to reference a folder unambiguously.
- Folders inherit the permissions of their vault. Access to the vault means access to all of its folders.

## Folder operations

Common operations you can perform on folders:

- **Create**: Make a new folder, at the vault root or under a parent folder
- **List**: View all folders in a vault, optionally filtered by parent
- **Update**: Rename a folder
- **Move**: Change a folder's parent, or move it to the vault root
- **Delete**: Remove a folder

## Examples

```bash
# Create a folder in a vault
pass-cli folder create --vault-name "Personal" --name "Banking"

# List folders to get their IDs
pass-cli folder list --vault-name "Personal"

# Create a nested folder using the parent folder ID
pass-cli folder create --vault-name "Personal" --name "Proton" --parent-id "folder_id_here"

# Create an item directly inside a folder
pass-cli item create login \
  --vault-name "Personal" \
  --title "Bank Login" \
  --username "myuser" \
  --generate-password \
  --folder-id "folder_id_here"
```
