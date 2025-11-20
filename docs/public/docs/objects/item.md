# Item object

An **Item** is the fundamental unit of data storage in Proton Pass. Items contain your sensitive information such as login credentials, secure notes, credit card details, and other personal data.

Items are identified by an item ID, but be aware that this ID is not necessarily unique. The only guarantee of uniqueness is that the ShareID + ItemID combination is globally unique.

## Item types

Proton Pass supports several types of items:

- **Login**: Username/email and password combinations for websites and services
- **Note**: Secure text notes for storing sensitive information
- **Credit Card**: Payment card information with secure storage
- **Identity**: Information about a person.
- **Alias**: Email aliases for privacy protection
- **SSH key**: Keys to access servers via SSH
- **Wifi**: Credentials to access a Wifi network

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

### Alias items

- **Email**: Email address to be used for receiving emails
- **Status**: Whether the forwarding process for emails sent to the address will be done or not
- **Mailboxes**: Actual email that will receive the emails

### Note items

- **Content**: The secure text content of the note

### Credit card items

- **Cardholder name**: Name on the card
- **Card number**: The credit card number (encrypted)
- **Expiry date**: When the card expires
- **CVV**: Security code (encrypted)

## Item sharing

Items can be shared in two ways:

1. **Vault sharing**: When a vault is shared, all items in it are accessible to vault members
2. **Individual item sharing**: Specific items can be shared with users who don't have access to the full vault

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

For detailed information about item references (uris starting with `pass://`), see the [item references](../commands/contents/item-references.md) documentation.
