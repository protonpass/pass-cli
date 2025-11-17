# `invite` command

Manage invitations in Proton Pass.

## Synopsis

```bash
pass-cli invite <SUBCOMMAND>
```

## Description

The `invite` command provides operations for managing invitations to access vaults and items. You can list pending invitations, accept or reject invitations, and manage group invitations.

## Subcommands

### list

List all pending invitations sent to you.

```bash
pass-cli invite list [--output FORMAT]
```

**Options:**
- `--output FORMAT` - Output format: `human` (default) or `json`

**Examples:**
```bash
# List pending invitations
pass-cli invite list

# List invitations in JSON format
pass-cli invite list --output json
```

### accept

Accept a pending invitation.

```bash
pass-cli invite accept --invite-token TOKEN
```

**Options:**
- `--invite-token TOKEN` - The invitation token to accept (required)

**Examples:**
```bash
# Accept an invitation
pass-cli invite accept --invite-token "abc123def456"
```

### reject

Reject a pending invitation.

```bash
pass-cli invite reject --invite-token TOKEN
```

**Options:**
- `--invite-token TOKEN` - The invitation token to reject (required)

**Examples:**
```bash
# Reject an invitation
pass-cli invite reject --invite-token "abc123def456"
```

### group

Manage group invitations.

```bash
pass-cli invite group <GROUP_SUBCOMMAND>
```

#### group list

List pending group invitations.

```bash
pass-cli invite group list [--output FORMAT]
```

**Options:**
- `--output FORMAT` - Output format: `human` (default) or `json`

**Examples:**
```bash
# List group invitations
pass-cli invite group list

# List group invitations in JSON format
pass-cli invite group list --output json
```

#### group accept

Accept a group invitation.

```bash
pass-cli invite group accept --invite-token TOKEN
```

**Options:**
- `--invite-token TOKEN` - The group invitation token to accept (required)

**Examples:**
```bash
# Accept a group invitation
pass-cli invite group accept --invite-token "group789xyz"
```

## Understanding invitations

### Types of invitations

#### Vault invitations
- **Purpose**: Grant access to an entire vault and all its items
- **Scope**: Current and future items in the vault
- **Roles**: Can be viewer, editor, or manager

#### Item invitations
- **Purpose**: Grant access to a specific item
- **Scope**: Only the shared item, not its parent vault
- **Roles**: Can be viewer, editor, or manager

#### Group invitations
- **Purpose**: Add you to a user group or organization
- **Scope**: May grant access to multiple resources
- **Management**: Centralized access management

### Invitation information

When listing invitations, you'll typically see:

- **Invitation type**: Vault, item, or group invitation
- **Resource name**: Name of the vault, item, or group
- **Inviter**: Who sent the invitation
- **Role offered**: Permission level being granted
- **Invitation date**: When the invitation was sent
- **Expiration**: When the invitation expires (if applicable)

## Examples

### Managing invitation workflow

```bash
# Check for new invitations
echo "=== Pending Invitations ==="
pass-cli invite list

# Check for group invitations
echo -e "\n=== Group Invitations ==="
pass-cli invite group list

# Accept specific invitations
pass-cli invite accept --invite-token "vault_invite_123"
pass-cli invite group accept --invite-token "group_invite_456"
```

## Invitation lifecycle

### Receiving invitations
1. **Notification**: You receive an invitation (email, in-app notification)
2. **Listing**: Use `pass-cli invite list` to see pending invitations
3. **Review**: Examine invitation details (resource, inviter, role)
4. **Decision**: Choose to accept or reject the invitation

### Processing invitations
1. **Acceptance**: Use `pass-cli invite accept` to gain access
2. **Rejection**: Use `pass-cli invite reject` to decline access
3. **Verification**: After confirming, run `pass-cli share list` to confirm new access
4. **Usage**: Access the shared resource through normal commands

### Expiration
- **Time limits**: Invitations may have expiration dates
- **Automatic cleanup**: Expired invitations are automatically removed
- **Re-invitation**: Inviters may need to send new invitations if they expire

## Best practices

### Security considerations
- **Verify inviter**: Ensure invitations come from trusted sources
- **Review permissions**: Check what role is being offered
- **Principle of least privilege**: Only accept invitations you actually need
- **Regular cleanup**: Process invitations promptly to avoid accumulation

## Troubleshooting

### Missing invitations

If you expect invitations that don't appear:

1. **Email verification**: Check if invitations were sent to the correct email
2. **Spam folder**: Check email spam/junk folders
3. **Account synchronization**: Try logging out and back in
4. **Inviter contact**: Contact the person who sent the invitation

### Invalid tokens

If invitation tokens don't work:

1. **Token accuracy**: Ensure you're using the complete, correct token
2. **Expiration**: Check if the invitation has expired
3. **Already processed**: Verify you haven't already accepted/rejected it
4. **Re-invitation**: Ask the inviter to send a new invitation

### Permission issues

If you can't access resources after accepting:

1. **Synchronization**: Allow time for permissions to propagate
2. **Role verification**: Check your role in `pass-cli share list`
3. **Resource status**: Ensure the shared resource still exists
4. **Re-authentication**: Try logging out and back in

## Related commands

- **[share](share.md)** - View and manage accepted shares
- **[vault](vault.md)** - Access shared vaults after accepting invitations
- **[item](item.md)** - Access shared items after accepting invitations
- **[user](user.md)** - View account information related to invitations
