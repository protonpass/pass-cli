# Vault object

A **Vault** is a container that organizes and groups your items in Proton Pass. Think of it as a folder or category that helps you organize your passwords, notes, and other sensitive information. Vaults help you organize items by purpose, project, or any other criteria. 

``` mermaid
flowchart TD
    subgraph g1 [ ]
    v1[("Vault 1")]
    i11@{ shape: notch-rect, label: "Login item in Vault 1"}
    i12@{ shape: notch-rect, label: "Card item in Vault 1"}
    end 
    subgraph g2 [ ]
    v2[("Vault 2")]
    i21@{ shape: notch-rect, label: "Note item in Vault 2"}
    i22@{ shape: notch-rect, label: "Identity item in Vault 2"}
    i23@{ shape: notch-rect, label: "Alias item in Vault 2"}
    end 
    v1 --> i11
    v1 --> i12
    v2 --> i21
    v2 --> i22
    v2 --> i23
```

Each vault can have a different set of items. But items can only exist in a single vault.

## Vault sharing

Vaults can be shared with other Proton Pass users:

- When you share a vault, the recipient gets access to all current and future items in the vault
- You can assign different roles (viewer, editor, manager) to control what the recipient can do

## Vault operations

Common operations you can perform on vaults:

- **Create**: Make a new vault to organize items
- **List**: View all vaults you have access to
- **Update**: Rename a vault
- **Delete**: Remove a vault and all its items (careful!)
- **Share**: Grant access to other users
- **Manage members**: Add, remove, or change roles of vault members

## Examples

```bash
# Create a new vault for work items
pass-cli vault create --name "Work Accounts"

# List all your vaults
pass-cli vault list

# Share a vault with a colleague
pass-cli vault share --share-id "abc123" colleague@company.com --role editor
```
