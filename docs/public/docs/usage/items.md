# Working with Items

Items are the actual credentials and data stored in vaults.

## List items in a vault

For listing items in a vault, you can use the Share ID of the vault.

```bash
pass-cli item list --share-id AbCdEf123456
```

Or you can specify the vault name:

```bash
pass-cli item list "Personal"
```

## Create a new item

In order to see which item types you can create, you can run:

```bash
pass-cli item create --help
```

Then, after you know which type of item you want to create, you can run `--help` again to see which options are available. Take into account that not all item types support the same creation options.

As an example, let's see a few ways for creating a new login item:

### Basic creation

```bash
pass-cli item create login \
  --share-id AbCdEf123456 \
  --title "GitHub Account" \
  --username "octocat" \
  --password "secret123" \
  --url "https://github.com"
```

### Generate a random password

```bash
pass-cli item create login \
  --share-id AbCdEf123456 \
  --title "New Account" \
  --username "user@example.com" \
  --generate-password
```

### Generate password with custom settings

```bash
pass-cli item create login \
  --share-id AbCdEf123456 \
  --title "New Account" \
  --username "user@example.com" \
  --generate-password=32,uppercase,symbols
```

Format: `length,uppercase,symbols` (numbers are always included)

### Generate a passphrase

```bash
pass-cli item create login \
  --share-id AbCdEf123456 \
  --title "New Account" \
  --username "user@example.com" \
  --generate-passphrase=5
```

This generates a passphrase with 5 words.

## Create from template

Get the JSON template:

```bash
pass-cli item create login --get-template
```

Output:

```json
{
  "title": "",
  "username": null,
  "email": null,
  "password": null,
  "urls": []
}
```

Create from template file:

```bash
cat > login.json << EOF
{
  "title": "Production Database",
  "username": "dbadmin",
  "password": "verysecure",
  "urls": ["https://db.example.com"]
}
EOF

pass-cli item create login \
  --share-id AbCdEf123456 \
  --from-template login.json
```

Or from stdin:

```bash
echo '{"title":"Test","username":"user","password":"pass","urls":[]}' | \
  pass-cli item create login \
    --share-id AbCdEf123456 \
    --from-template -
```

## View an item

The CLI can print the full details of an item by specifying both the Share ID and the Item ID:

```bash
pass-cli item view --share-id AbCdEf123456 --item-id XyZ789
```

And also by specifying the path in a URI format:

```bash
pass-cli item view "pass://Personal/TestItem"
```

## View an item field

The CLI can print a single field of an item by specifying the Share ID, the Item ID and the field name:

```bash
pass-cli item view --share-id AbCdEf123456 --item-id XyZ789 --field password
```

And also by specifying the path in a URI format:

```bash
pass-cli item view "pass://Personal/TestItem/password"
```

## Generate TOTP code(s)

The CLI can generate TOTP (Time-based One-Time Password) codes for items that have TOTP fields.

### Generate TOTP for a specific field

```bash
pass-cli item totp --share-id AbCdEf123456 --item-id XyZ789 --field totp
```

### Generate all TOTP codes in an item

```bash
pass-cli item totp --share-id AbCdEf123456 --item-id XyZ789
```

This will find and generate codes for all TOTP fields in the item.

### Using vault and item names

```bash
pass-cli item totp --vault-name Personal --item-title "GitHub Account"
```

### Using Pass URI format

```bash
pass-cli item totp "pass://Personal/GitHub Account/totp"
```

### JSON output

```bash
pass-cli item totp --vault-name Personal --item-title "GitHub Account" --output json
```

Output:
```json
{
  "totp": "123456"
}
```

## Delete an item

```bash
pass-cli item delete --share-id AbCdEf123456 --item-id XyZ789
```

