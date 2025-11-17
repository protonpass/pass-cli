# Vault object

A **Vault** is a container that organizes and groups your items in Proton Pass. Think of it as a folder or category that helps you organize your passwords, notes, and other sensitive information.

## Key characteristics

- **Container for items**: A vault contains multiple items (logins, notes, credit cards, etc.)
- **Organizational unit**: Vaults help you organize items by purpose, project, or any other criteria
- **Sharing boundary**: When you share a vault, you grant access to all items within it
- **Access control**: Each vault has its own set of permissions and members

## Vault properties

Each vault has the following properties:

- **Name**: A human-readable name for the vault
- **Share ID**: A unique identifier used for API operations
- **Members**: Users who have access to the vault and their roles
- **Items**: The collection of items stored within the vault

## Vault sharing

Vaults can be shared with other Proton Pass users:

- When you share a vault, you create a **Share** relationship between the user and the vault
- The recipient gets access to all current and future items in the vault
- You can assign different roles (viewer, editor, manager) to control what the recipient can do

## Relationship with items

- **One-to-many**: A vault can contain many items
- **Exclusive ownership**: Each item belongs to exactly one vault
- **Inheritance**: Items inherit access permissions from their parent vault

## Common use cases

- **Personal vault**: Your default vault for personal passwords and accounts
- **Work vault**: Separate vault for work-related credentials
- **Project vault**: Vault shared with team members for a specific project
- **Family vault**: Shared vault for household accounts and services

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
