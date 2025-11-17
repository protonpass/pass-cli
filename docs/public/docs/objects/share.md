# Share object

A **Share** represents the relationship between a user and a resource in Proton Pass. It defines what access a user has to either a vault or an individual item, and what permissions they have to interact with that resource.

All shares have a single unique ID, which is a long identifier. Most of the Proton Pass CLI commands need to act on the share that links the user to the resource, and they always accept the share ID as an argument. However, some commands may also offer the option to type the resource name, although in case of duplicates, there is no guarantee that it will act on the one you expected. So in case of doubt, always prefer referring to resource by its corresponding ID.

When sharing a resource, a new Share instance will be created for the target user. That means, if you create a vault, you will have a share with some ID, and if you share it with another user, they will also have a share pointing to that vault, but the ID will be different.

## Key characteristics

- **Access relationship**: A share grants a user access to a specific resource
- **Permission control**: Shares define what actions the user can perform
- **Role-based**: Each share has an associated role that determines permissions

## Types of shares

### Vault shares
- **Resource**: An entire vault and all items within it
- **Scope**: Access to all current and future items in the vault
- **Creation**: When you create a new vault, or when someone shares a vault with you
- **Impact**: You can see and interact with the entire vault

### Item shares  
- **Resource**: A single, specific item
- **Scope**: Access only to that particular item
- **Creation**: When someone shares an individual item with you
- **Impact**: You can see only that item, not its parent vault nor any other item contained in that vault

## Share roles

Each share has a role that determines what the user can do:

### Viewer
- **Read access**: Can view the resource and its contents
- **No modifications**: Cannot edit, delete, or share the resource
- **Safe access**: Perfect for read-only access to sensitive information

### Editor
- **Read and write**: Can view and modify the resource
- **Item management**: In vault shares they can create, edit, and delete items (in vault shares). In item shares they can only update or delete the item.
- **No sharing**: Cannot share the resource with others
- **No vault management**: Cannot delete vaults or manage members

### Manager
- **Full control**: Can perform all operations on the resource
- **Sharing rights**: Can share the resource with other users
- **Member management**: Can add, remove, and change roles of other users (except for the vault owner)
- **Administrative**: Only the vault owner (the user who created it) can delete the vault

## Share lifecycle

### Creation
1. A user (owner or manager) decides to share a resource
2. They specify the target user's email address
3. They assign a role (viewer, editor, manager)
4. An invitation is sent to the target user
5. The target user accepts the invitation
6. A share is created, granting access

### Management
- **Role changes**: Managers can modify user roles
- **Access revocation**: Managers can remove shares
- **Permission inheritance**: Vault shares affect all contained items

### Termination
- **Explicit removal**: A manager removes the share
- **Resource deletion**: The shared resource is deleted
- **Account changes**: User accounts are deactivated

## Examples

### List your available shares
```bash
pass-cli share list
- [ABCDEFGHIJKL==] Type=Vault | Role=Owner | MyVault
- [ZYXWVUTSRQPO==] Type=Item | Role=Viewer | SomeItem
```

### Vault sharing scenario
```bash
# Alice shares her "Work Projects" vault with Bob as an editor
pass-cli vault share --share-id "WorkProjectsShareForAlice" bob@company.com --role editor

# Bob can now see all items in the vault and create new ones
pass-cli vault list  # Shows "Work Projects" vault
pass-cli item list --share-id "WorkProjectsShareForBob"  # Shows all items in the vault
```

### Item sharing scenario
```bash
# Alice shares a specific login item with Charles as a viewer
pass-cli item share --share-id "WorkProjectsShareForAlice" --item-id "login456" charles@company.com --role viewer

# Charles can view this specific item but not the entire vault
pass-cli item view --share-id "WorkProjectsItemShareForCharles" --item-id "login456"
```

## Best practices

- **Principle of least privilege**: Grant the minimum necessary permissions
- **Regular audits**: Periodically review who has access to what
- **Role appropriateness**: Choose roles based on actual needs
- **Vault vs item**: Use vault shares for ongoing collaboration, item shares for specific access
- **Documentation**: Keep track of why access was granted and when it might be removed
