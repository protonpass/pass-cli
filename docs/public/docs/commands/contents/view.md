# `view` command

View an item's details.

```bash
pass-cli item view [OPTIONS] [URI]
```

**Options:**

- `--share-id SHARE_ID` - Share ID of the vault containing the item
- `--vault-name VAULT_NAME` - Name of the vault containing the item
- `--item-id ITEM_ID` - ID of the item to view
- `--item-title ITEM_TITLE` - Title of the item to view
- `URI` - Secret reference as specified in [here](secret-references.md)
- `--output FORMAT` - Output format: `human` (default) or `json`

**Mutually exclusive options:**

- `--share-id` and `--vault-name` are mutually exclusive. You must provide exactly one.
- `--item-id` and `--item-title` are mutually exclusive. You must provide exactly one.
- `--share-id/--vault-name` and `--item-id/--item-title` parameters and `URI` are mutually exclusive. You must provide either both parameters or a single secret reference.

**Examples:**

```bash
# View item by IDs
pass-cli item view --share-id "abc123def" --item-id "item456"

# View item by vault name and item title
pass-cli item view --vault-name "MyVault" --item-title "MyItem"

# View item using Pass URI
pass-cli item view "pass://abc123def/item456"

# View specific field using URI
pass-cli item view "pass://abc123def/item456/password"

# View specific field using options
pass-cli item view --share-id "abc123def" --item-id "item456" --field "username"

# View item in JSON format
pass-cli item view --share-id "abc123def" --item-id "item456" --output json
```

