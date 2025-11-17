# Item object

An **Item** is the fundamental unit of data storage in Proton Pass. Items contain your sensitive information such as login credentials, secure notes, credit card details, and other personal data.

Items are identified by an item ID, but be aware that this ID is not necessarily unique! The only guarantee of uniqueness is that the ShareID + ItemID combination is globally unique. 

## Key characteristics

- **Belongs to a vault**: Every item must be stored within exactly one vault
- **Typed data**: Items have specific types (login, note, credit card, etc.)
- **Encrypted storage**: All item data is encrypted and secure
- **Individual sharing**: Items can be shared independently of their vault

## Item types

Proton Pass supports several types of items:

- **Login**: Username/email and password combinations for websites and services
- **Note**: Secure text notes for storing sensitive information
- **Credit Card**: Payment card information with secure storage
- **Alias**: Email aliases for privacy protection

## Item properties

All items share common properties:

- **Title**: A descriptive name for the item
- **Item ID**: An identifier for API operations. Remember that this Item ID is only unique in combination with the Share ID
- **Vault**: The vault that contains this item
- **Type**: The specific type of item (login, note, etc.)
- **Creation/modification dates**: When the item was created and last updated

## Type-specific properties

### Login items

- **Username/Email**: Account identifier
- **Password**: Account password
- **URLs**: Associated websites or services
- **TOTP**: Two-factor authentication codes (if configured)
- **Custom fields**: Additional structured data

### Alias items

- **Email**: Email address to be used for receiving emails
- **Status**: Whether the forwarding process for emails sent to the address will be done or not
- **Mailboxes**: Actual email that will receive the emails
- **Custom fields**: Additional structured data

### Note items

- **Content**: The secure text content of the note
- **Custom fields**: Additional structured data

### Credit card items

- **Cardholder name**: Name on the card
- **Card number**: The credit card number (encrypted)
- **Expiry date**: When the card expires
- **CVV**: Security code (encrypted)
- **Custom fields**: Additional information

## Item sharing

Items can be shared in two ways:

1. **Vault sharing**: When a vault is shared, all items in it are accessible to vault members
2. **Individual item sharing**: Specific items can be shared with users who don't have access to the full vault

When you share an item individually:
- A **Share** relationship is created between the user and the specific item
- The recipient can access only that item, not the entire vault
- You can control the recipient's permissions (view, edit, etc.)

## Item operations

Common operations you can perform on items:

- **Create**: Add new items to a vault
- **List**: View items in a vault or across all vaults
- **View**: Display item details and field values
- **Update**: Modify item information
- **Delete**: Remove an item permanently
- **Share**: Grant access to specific users

## URI format

Items can be referenced using Proton Pass URIs:

```
pass://SHARE_ID/ITEM_ID[/FIELD]
```

- **SHARE_ID**: The unique vault share identifier, or the vault name
- **ITEM_ID**: The item identifier, or the item name  
- **FIELD**: Optional specific field (e.g., "password", "username")

## Examples

```bash
# Create a new login item
pass-cli item create login \
  --share-id "vault123" \
  --title "GitHub Account" \
  --username "myuser" \
  --password "mypassword" \
  --url "https://github.com"

# View an item using URI
pass-cli item view "pass://vault123/item456"

# View just the password field
pass-cli item view "pass://vault123/item456/password"

# List all items in a vault
pass-cli item list --share-id "vault123"
```

