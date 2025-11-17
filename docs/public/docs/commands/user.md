# `user` command

Manage user account information and operations.

## Synopsis

```bash
pass-cli user <SUBCOMMAND>
```

## Description

The `user` command provides operations for viewing and managing your Proton Pass user account information, including profile details, account status, and user-specific settings.

## Subcommands

### info

Display detailed information about your user account.

```bash
pass-cli user info [--output FORMAT]
```

**Options:**
- `--output FORMAT` - Output format: `human` (default) or `json`

**Examples:**
```bash
# Display user information in human-readable format
pass-cli user info

# Display user information in JSON format
pass-cli user info --output json
```

## User information

The `user info` command typically displays:

### Account details
- **Email address**: Your Proton account email
- **Account status**: Whether your account is active, suspended, etc.
- **Account type**: Free, paid, or business account
- **Subscription information**: Current plan and features

### Profile information
- **Display name**: Your account display name
- **Account creation date**: When your account was created
- **Last login**: When you last accessed Proton Pass
- **Account settings**: Various account preferences

### Usage statistics
- **Vault count**: Number of vaults you own
- **Item count**: Total number of items across all vaults
- **Share count**: Number of resources you've shared
- **Storage usage**: Amount of storage used (if applicable)

### Feature availability
- **Available features**: List of features available to your account
- **Limits**: Any usage limits based on your subscription
- **Permissions**: Account-level permissions and capabilities

## Examples

### Basic user information

```bash
# View your account details
pass-cli user info
```

### Account verification script

```bash
#!/bin/bash
echo "=== Account Verification ==="

# Get user info in JSON format
USER_INFO=$(pass-cli user info --output json)

# Extract key information
EMAIL=$(echo "$USER_INFO" | jq -r '.email')
STATUS=$(echo "$USER_INFO" | jq -r '.status')
PLAN=$(echo "$USER_INFO" | jq -r '.plan')

echo "Email: $EMAIL"
echo "Status: $STATUS"  
echo "Plan: $PLAN"

# Verify account is active
if [ "$STATUS" = "active" ]; then
    echo "✓ Account is active"
else
    echo "⚠ Account status: $STATUS"
fi
```

### Usage monitoring

```bash
#!/bin/bash
echo "=== Usage Report ==="

# Get detailed usage information
pass-cli user info --output json | jq '{
  email: .email,
  plan: .plan,
  usage: {
    vaults: .vault_count,
    items: .item_count,
    shares: .share_count,
    storage: .storage_used
  },
  limits: {
    max_vaults: .max_vaults,
    max_items: .max_items,
    max_storage: .max_storage
  }
}'
```

### Account health check

```bash
#!/bin/bash
echo "=== Account Health Check ==="

# Check account status
if pass-cli user info > /dev/null 2>&1; then
    echo "✓ Account accessible"
    
    # Get account status
    STATUS=$(pass-cli user info --output json | jq -r '.status')
    if [ "$STATUS" = "active" ]; then
        echo "✓ Account is active"
    else
        echo "⚠ Account status: $STATUS"
    fi
    
    # Check for any warnings or issues
    WARNINGS=$(pass-cli user info --output json | jq -r '.warnings[]? // empty')
    if [ -n "$WARNINGS" ]; then
        echo "⚠ Account warnings:"
        echo "$WARNINGS"
    else
        echo "✓ No account warnings"
    fi
    
else
    echo "✗ Cannot access account information"
    echo "  Try: pass-cli login your@email.com"
fi
```

## Use cases

### Account validation

Verify your account details and status:

```bash
# Quick account check
pass-cli user info | grep -E "(Email|Status|Plan)"

# Detailed account validation
pass-cli user info --output json | jq '{email, status, plan, features}'
```

### Subscription monitoring

Monitor your subscription and usage:

```bash
#!/bin/bash
# Subscription monitoring script

USER_DATA=$(pass-cli user info --output json)

echo "=== Subscription Status ==="
echo "Plan: $(echo "$USER_DATA" | jq -r '.plan')"
echo "Status: $(echo "$USER_DATA" | jq -r '.subscription_status')"
echo "Renewal: $(echo "$USER_DATA" | jq -r '.renewal_date')"

echo -e "\n=== Usage vs Limits ==="
VAULT_COUNT=$(echo "$USER_DATA" | jq -r '.vault_count')
MAX_VAULTS=$(echo "$USER_DATA" | jq -r '.max_vaults')
echo "Vaults: $VAULT_COUNT / $MAX_VAULTS"

ITEM_COUNT=$(echo "$USER_DATA" | jq -r '.item_count')  
MAX_ITEMS=$(echo "$USER_DATA" | jq -r '.max_items')
echo "Items: $ITEM_COUNT / $MAX_ITEMS"
```

### Feature availability check

Check what features are available to your account:

```bash
#!/bin/bash
echo "=== Feature Availability ==="

# List available features
pass-cli user info --output json | jq -r '.features[]' | while read feature; do
    echo "✓ $feature"
done

# Check for premium features
PREMIUM_FEATURES=$(pass-cli user info --output json | jq -r '.premium_features[]? // empty')
if [ -n "$PREMIUM_FEATURES" ]; then
    echo -e "\n=== Premium Features ==="
    echo "$PREMIUM_FEATURES" | while read feature; do
        echo "⭐ $feature"
    done
fi
```

### Multi-account management

When managing multiple accounts:

```bash
#!/bin/bash
# Multi-account status check

ACCOUNTS=("alice@company.com" "bob@company.com" "carol@company.com")

for account in "${ACCOUNTS[@]}"; do
    echo "=== Checking $account ==="
    
    # Login and check account
    if pass-cli login "$account"; then
        pass-cli user info | grep -E "(Email|Status|Plan)"
        pass-cli logout
    else
        echo "Failed to login to $account"
    fi
    
    echo
done
```

## Integration with other commands

### Cross-reference with shares

```bash
#!/bin/bash
echo "=== Account and Sharing Overview ==="

# User account info
echo "Account: $(pass-cli user info --output json | jq -r '.email')"
echo "Plan: $(pass-cli user info --output json | jq -r '.plan')"

# Owned resources
echo -e "\nOwned vaults: $(pass-cli vault list --output json | length)"

# Shared resources
echo "Shared with me: $(pass-cli share list --output json | length)"
```

### Account-based automation

```bash
#!/bin/bash
# Account-aware automation

USER_PLAN=$(pass-cli user info --output json | jq -r '.plan')

if [ "$USER_PLAN" = "free" ]; then
    echo "Free account detected - using basic features"
    # Limited functionality
elif [ "$USER_PLAN" = "premium" ]; then
    echo "Premium account detected - using advanced features"
    # Full functionality
fi
```

## Privacy and security

### Information sensitivity

The `user info` command shows:
- ✅ **Safe to display**: Email, plan type, feature availability
- ⚠️ **Potentially sensitive**: Usage statistics, account IDs
- 🔒 **Never shown**: Passwords, private keys, payment information

### Best practices
- **Regular monitoring**: Check account status regularly
- **Anomaly detection**: Watch for unexpected changes in usage or status
- **Security alerts**: Monitor for any security-related warnings
- **Access logging**: Keep records of when account information was accessed

## Troubleshooting

### Cannot access user info

If the command fails:

1. **Authentication**: Ensure you're logged in with `pass-cli login`
2. **Network**: Check internet connectivity
3. **Account status**: Your account might be suspended or locked

### Outdated information

If information seems outdated:

1. **Cache refresh**: Try logging out and back in
2. **Synchronization**: Allow time for account changes to propagate
3. **Browser comparison**: Compare with web interface information

### Missing features

If expected features aren't shown:

1. **Plan verification**: Confirm your subscription plan
2. **Feature rollout**: Some features may be gradually rolled out
3. **Account type**: Business vs personal accounts may have different features

## Related commands

- **[login](login.md)** - Authenticate to access user information
- **[info](info.md)** - Session-specific information (different from user info)
- **[share](share.md)** - View resources shared with your user account
- **[vault](vault.md)** - Manage vaults owned by your user account
