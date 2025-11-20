# Item references

Item references are a way to reference secrets stored in your Proton Pass vaults without exposing the actual values. They use a URL-like syntax (`pass://`) that can be resolved at runtime by the CLI.

## What are item references?

Item references are placeholders that point to specific fields in items stored in your Proton Pass vaults. Instead of hardcoding passwords, API keys, or other sensitive data, you can use a reference that the CLI will resolve to the actual secret value when needed.

## Syntax

Item references use the following format:

```text
pass://<vault-identifier>/<item-identifier>/<field-name>
```

Where:

- **`vault-identifier`**: The vault's Share ID or name
- **`item-identifier`**: The item's ID or title
- **`field-name`**: The specific field to retrieve (e.g., `password`, `username`, `email`, `url`, `note`, or custom field names)

## How it works

1. **Reference creation**: You write a `pass://` URI in your configuration files or environment variables
2. **Resolution**: When you use the [`run`](run.md) or [`inject`](inject.md) commands, the CLI:
   - Parses the reference to identify the vault, item, and field
   - Resolves vault/item names to their IDs if needed
   - Fetches the actual secret value from Proton Pass
   - Replaces the reference with the secret value
3. **Usage**: Your application receives the actual secret value, not the reference

## Examples

### Basic references

```text
pass://Work/GitHub/password
pass://Personal/Email Login/username
pass://AbCdEf123456/XyZ789/password
pass://My Vault/My Item/My Custom Field
```

### Using names vs IDs

You can reference vaults and items by either their names or IDs:

**By name:**

```text
pass://Work/GitHub Account/password
pass://Personal/Email Login/username
```

**By ID:**

```text
pass://AbCdEf123456/XyZ789/password
pass://ShareId123/ItemId456/api_key
```

**Mixed:**

```text
pass://Work/XyZ789/password          # Vault by name, item by ID
pass://AbCdEf123456/GitHub/password  # Vault by ID, item by name
```

## Field names

### Common fields

For login items, common fields include:

- `username` - The username/login name
- `password` - The password
- `email` - Email address
- `url` - Website URL
- `note` - Additional notes
- `totp` - TOTP secret (for two-factor authentication)

### Custom fields

Items can have custom fields with any name. The field name must match exactly (case-sensitive).

```text
pass://Work/API Keys/api_key
pass://Production/Database/connection_string
pass://Services/Stripe/secret_key
```

## Rules and limitations

### Required components

- **Vault identifier**: Must be provided (Share ID or vault name)
- **Item identifier**: Must be provided (Item ID or item title)
- **Field name**: Must be provided (field names are case-sensitive)

### Name resolution

- Names with spaces are supported: `pass://My Vault/My Item/password`
- Names are resolved to IDs internally by the CLI
- If multiple vaults/items have the same name, the first match is used
- Resolution is case-sensitive

### Invalid formats

The following are **not** valid secret references:

```text
pass://vault/item              # Missing field name
pass://vault/item/             # Trailing slash
pass://vault/                  # Missing item and field
pass://                        # Empty reference
```

## Usage with commands

### With `view` command

The [`view`](view.md) command displays item contents:

```bash
export DB_PASSWORD='pass://Production/Database/password'
pass-cli view $DB_PASSWORD
```

### With `run` command

The [`run`](run.md) command resolves secret references in environment variables:

```bash
export DB_PASSWORD='pass://Production/Database/password'
pass-cli run -- ./my-app
```

The application receives the actual password value.

### With `inject` command

The [`inject`](inject.md) command resolves secret references in template files:

```yaml
# config.yaml.template
database:
  password: {{ pass://Production/Database/password }}
```

After injection:

```yaml
database:
  password: actual_secret_value_here
```

**Note**: With `inject`, references must be wrapped in double braces: `{{ pass://... }}`

## Troubleshooting

### Reference not found

If a reference cannot be resolved:

1. **Check vault access**: Verify you have access to the vault

   ```bash
   pass-cli vault list
   ```

2. **Check item exists**: Verify the item exists in the vault

   ```bash
   pass-cli item list --share-id <vault-share-id>
   ```

3. **Verify field name**: Check the exact field name (case-sensitive)

   ```bash
   pass-cli item view --share-id <share-id> --item-id <item-id>
   ```

4. **Check name resolution**: If using names, ensure they're spelled correctly

### Common errors

**"Invalid reference format"**

- Ensure the reference follows `pass://vault/item/field` format
- Check for trailing slashes
- Verify all three components are present

**"Secret reference requires a field name"**

- Add the field name: `pass://vault/item/field` (not `pass://vault/item`)

**"Field not found"**

- Verify the field exists in the item
- Check the field name is spelled correctly (case-sensitive)
- Use `pass-cli item view` to see available fields

## Related commands

- **[run](run.md)** - Execute commands with secrets injected from references
- **[inject](inject.md)** - Process template files with secret references
- **[item view](item.md#view)** - View item details to see available fields
- **[vault list](vault.md#list)** - List vaults to find Share IDs
