# `view` command

View an item's details.

```bash
pass-cli item view [OPTIONS] [URI]
```

**Options:**

- `--share-id SHARE_ID` - Share ID of the vault containing the item
- `--item-id ITEM_ID` - ID of the item to view
- `URI` - Item reference in as specified in [here](item-references.md)
- `--field FIELD` - Specific field to view
- `--output FORMAT` - Output format: `human` (default) or `json`

**Examples:**

```bash
# View item by IDs
pass-cli item view --share-id "abc123def" --item-id "item456"

# View item using Pass URI
pass-cli item view "pass://abc123def/item456"

# View specific field using URI
pass-cli item view "pass://abc123def/item456/password"

# View specific field using options
pass-cli item view --share-id "abc123def" --item-id "item456" --field "username"

# View item in JSON format
pass-cli item view --share-id "abc123def" --item-id "item456" --output json
```

