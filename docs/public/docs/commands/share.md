# `share` command

Manage shares in Proton Pass.

## Synopsis

```bash
pass-cli share <SUBCOMMAND>
```

## Description

The `share` command provides operations for managing shares, which represent access relationships between users and resources (vaults or items). You can list and manage all types of shares you have access to.

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

## Understanding shares

A share represents the relationship between a user and a resource. There are two types of shares:

### Vault shares
- **Resource**: Access to an entire vault and all items within it
- **Scope**: Current and future items in the vault
- **Inheritance**: Items inherit vault access permissions

### Item shares  
- **Resource**: Access to a single, specific item
- **Scope**: Only the shared item, not its parent vault
- **Independence**: Separate from vault-level permissions

## Share information

When listing shares, you'll typically see:

- **Resource type**: Whether it's a vault or item share
- **Resource name**: The name of the vault or item
- **Owner**: Who originally shared the resource with you
- **Role**: Your permission level (viewer, editor, manager)
- **Share date**: When the resource was shared with you

## Share roles

Each share has an associated role that determines your permissions:

### Viewer
- **Read access**: Can view the resource and its contents
- **No modifications**: Cannot edit, delete, or share
- **Safe access**: Perfect for read-only requirements

### Editor
- **Read and write**: Can view and modify the resource
- **Content management**: Can create, edit, and delete items (in vault shares)
- **Limited sharing**: Cannot share with others or manage members

### Manager
- **Full control**: Can perform all operations on the resource
- **Sharing rights**: Can share the resource with other users
- **Administrative**: Can manage members and their roles

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
echo "Vault shares: $(jq '[.[] | select(.type == "vault")] | length' shares.json)"
echo "Item shares: $(jq '[.[] | select(.type == "item")] | length' shares.json)"

# List manager permissions
echo "Resources where you're a manager:"
jq -r '.[] | select(.role == "manager") | .name' shares.json

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
  .[] | 
  "Type: \(.type | ascii_upcase) | Name: \(.name) | Role: \(.role | ascii_upcase) | Owner: \(.owner)"
' | sort

echo
echo "=== Summary ==="
pass-cli share list --output json | jq -r '
  group_by(.type) | 
  map("- \(.[0].type | ascii_upcase) shares: \(length)") | 
  .[]
'
```

## Use cases

### Access inventory

Understanding what resources you have access to:

```bash
# Quick overview of all shared resources
pass-cli share list

# Detailed analysis
pass-cli share list --output json | jq '.[] | {name, type, role, owner}'
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
  jq -r '.[] | select(.role == "manager") | "- \(.name) (\(.type))"'

echo -e "\n=== Editor Permissions ==="  
pass-cli share list --output json | \
  jq -r '.[] | select(.role == "editor") | "- \(.name) (\(.type))"'

echo -e "\n=== Viewer Permissions ==="
pass-cli share list --output json | \
  jq -r '.[] | select(.role == "viewer") | "- \(.name) (\(.type))"'
```

### Compliance reporting

Generate reports for compliance or security reviews:

```bash
#!/bin/bash
# Generate access report for security team

OUTPUT_FILE="access_report_$(date +%Y%m%d).json"

echo "Generating access report: $OUTPUT_FILE"

pass-cli share list --output json | jq '{
  report_date: now | strftime("%Y-%m-%d %H:%M:%S"),
  user: env.USER,
  shares: map({
    resource_name: .name,
    resource_type: .type,
    permission_level: .role,
    shared_by: .owner,
    access_granted: .shared_date
  })
}' > "$OUTPUT_FILE"

echo "Report saved to: $OUTPUT_FILE"
```

## Integration with other commands

### Cross-reference with vaults

```bash
# Compare vault list with share list
echo "=== Vaults you own ==="
pass-cli vault list

echo -e "\n=== Vaults shared with you ==="
pass-cli share list --output json | \
  jq -r '.[] | select(.type == "vault") | "- \(.name) (Role: \(.role))"'
```

### Identify accessible items

```bash
#!/bin/bash
# Show items accessible through different share types

echo "=== Items in owned vaults ==="
pass-cli vault list --output json | \
  jq -r '.[] | .share_id' | \
  while read share_id; do
    echo "Vault: $(pass-cli vault list --output json | jq -r ".[] | select(.share_id == \"$share_id\") | .name")"
    pass-cli item list --share-id "$share_id"
  done

echo -e "\n=== Directly shared items ==="
pass-cli share list --output json | \
  jq -r '.[] | select(.type == "item") | "- \(.name) (Role: \(.role))"'
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

### Outdated information

If share information seems outdated:

1. **Refresh**: Try logging out and back in
2. **Network**: Check network connectivity
3. **Synchronization**: Allow time for changes to propagate

## Related commands

- **[vault](vault.md)** - Manage vaults and vault sharing
- **[item](item.md)** - Manage items and item sharing  
- **[invite](invite.md)** - Manage pending invitations
- **[user](user.md)** - View user account information
