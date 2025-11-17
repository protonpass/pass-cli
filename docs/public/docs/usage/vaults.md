# Working with Vaults

Vaults are containers for items. Each vault has a unique ID and is accessed through a unique share ID.

You can think of a Share as the relationship between a User and a Resource.
Many users can access the same vault (with the same VaultID), but each of them will do it based on their own Share.
Resources can be either Vaults or Items (a user can share with another user access to an entire Vault, or just to a specific Item).

## List all vaults

```bash
pass-cli vault list
```

Output shows:

- Vault name
- Share ID (needed for other operations)
- Item count
- Owner information

## Create a vault

```bash
pass-cli vault create "Work Passwords"
```

This creates a new vault and returns its share ID:

```text
Created vault with id: AbCdEf123456
```

**Note:** A default vault named "Personal" is automatically created on first login if no vaults exist.

## Get share list

```bash
pass-cli share list
```

This shows detailed information about all shares you have access to, both for Vaults and also Items shared with you.

