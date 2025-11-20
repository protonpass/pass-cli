# `share` command

Manage shares in Proton Pass.

!!! info "Sharing content in Proton Pass"
    If you are looking for a way to share content in Proton Pass, please refer to the [`vault share`](./vault.md#share) and the [`item share`](./item.md#share) commands.


## Synopsis

```bash
pass-cli share <SUBCOMMAND>
```

## Description

The `share` command provides operations for managing shares, which represent access relationships between users and resources (vaults or items). You can list and manage all types of shares you have access to.

A share represents the relationship between a user and a resource. For information regarding the **Share** concept, please look at the [Share reference](../objects/share.md).

## Subcommands

### list

List all shares you have access to.

```bash
pass-cli share list [--output FORMAT]
```

**Options:**

- `--output FORMAT` - Output format: `human` (default) or `json`

**Examples:**

```bash
# List all shares in human-readable format
pass-cli share list

# List shares in JSON format for scripting
pass-cli share list --output json
```

## Examples

### List all shares

```bash
# See all resources shared with you
pass-cli share list
```

### Analyze share permissions

```bash
#!/bin/bash
echo "=== Share Analysis ==="

# List shares in JSON for processing
pass-cli share list --output json > shares.json

# Count different types of shares
echo "Vault shares: $(jq '[.shares[] | select(.share_type == "Vault")] | length' shares.json)"
echo "Item shares: $(jq '[.shares[] | select(.share_type == "Item")] | length' shares.json)"

# List manager permissions
echo "Resources where you're a manager:"
jq -r '.shares[] | select(.share_role == "Manager") | .name' shares.json

echo "Your owned resources:"
jq -r '.shares[] | select(.share_role == "Owner") | .name' shares.json

rm shares.json
```

### Share audit script

```bash
#!/bin/bash
echo "=== Share Audit Report ==="
echo "Generated: $(date)"
echo

# List all shares
pass-cli share list --output json | jq -r '
  .shares[] |
  "Type: \(.share_type | ascii_upcase) | Name: \(.name) | Role: \(.share_role | ascii_upcase)"
' | sort

echo
echo "=== Summary ==="
pass-cli share list --output json | jq -r '
  .shares
  | group_by(.share_type)
  | map("- \((.[0].share_type | ascii_upcase)) shares: \(length)")
  | .[]
'
```

## Use cases

### Access inventory

Understanding what resources you have access to:

```bash
# Quick overview of all shared resources
pass-cli share list

# Detailed analysis
pass-cli share list --output json | jq '.shares[] | {name, share_type, share_role}'
```

### Permission audit

Regular audits of your access permissions:

```bash
#!/bin/bash
# Monthly access review

echo "=== Monthly Access Review ==="
echo "Date: $(date)"

echo -e "\n=== Manager Permissions ==="
pass-cli share list --output json | \
  jq -r '.shares[] | select(.share_role == "Manager") | "- \(.name) (\(.share_type))"'

echo -e "\n=== Editor Permissions ==="  
pass-cli share list --output json | \
  jq -r '.shares[] | select(.share_role == "Editor") | "- \(.name) (\(.share_type))"'

echo -e "\n=== Viewer Permissions ==="
pass-cli share list --output json | \
  jq -r '.shares[] | select(.share_role == "Viewer") | "- \(.name) (\(.share_type))"'
```

## Best practices

### Regular reviews

- **Monthly audits**: Review your shares monthly
- **Permission validation**: Ensure you still need access to shared resources
- **Role appropriateness**: Verify your roles match your actual needs

### Security considerations

- **Principle of least privilege**: You should have the minimum necessary permissions
- **Access documentation**: Keep track of why you have access to resources
- **Removal requests**: Request removal of access you no longer need

### Organization

- **Categorization**: Group shares by purpose or project
- **Documentation**: Maintain records of share purposes
- **Communication**: Stay in touch with resource owners about access needs

## Troubleshooting

### Missing expected shares

If you don't see expected shares:

1. **Invitation status**: Check if you've accepted invitations
2. **Email verification**: Ensure invitations were sent to the correct email
3. **Account synchronization**: Try logging out and back in

### Permission issues

If you can't perform expected operations:

1. **Role verification**: Check your role in the share list
2. **Resource type**: Verify if it's a vault or item share
3. **Owner contact**: Contact the resource owner for permission changes

## Related commands

- **[vault](vault.md)** - Manage vaults and vault sharing
- **[item](item.md)** - Manage items and item sharing  
- **[invite](invite.md)** - Manage pending invitations
- **[user](user.md)** - View user account information
