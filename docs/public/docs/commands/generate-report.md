# `user generate-report` command

Generate a report on usage and security metrics for members of your Proton Pass organization.

## Synopsis

```bash
pass-cli user generate-report [--output FORMAT]
```

## Description

The `user generate-report` command retrieves organization member reports from the Proton Pass API, including breach counts, vault and item counts, password security metrics, and last activity. It fetches all pages automatically and displays the combined report.

The command retrieves existing metrics; it does not trigger a new security scan.

## Options

- `--output FORMAT` - Output format: `human` (default) or `json`

## Information displayed

- **Total members**: Organization member count reported by the API
- **User**: Member's primary email address
- **Breaches**: Number of affected emails and total breaches, listed separately for email aliases and addresses
- **Owned vaults/items**: Number of vaults and items the member owns
- **Accessible vaults/items**: Number of vaults and items the member can access
- **Password security**: Counts of reused, weak, and compromised passwords, plus inactive 2FA
- **Last activity**: Date in UTC (`YYYY-MM-DD`), or `never` when the timestamp is zero

JSON output also includes user and organization IDs, excluded item counts, report timestamps, and client versions. Timestamps are Unix seconds.

## Examples

```bash
# Display the organization report
pass-cli user generate-report

# Display the full report in JSON format
pass-cli user generate-report --output json

# Save the JSON report to a file
pass-cli user generate-report --output json > organization-report.json
```

### JSON output

Example response:

```json
{
  "member_reports": [
    {
      "primary_email": "user@example.com",
      "custom_email_breach_count": {
        "breach_email_count": 2,
        "total_breach_count": 7
      },
      "address_breach_count": {
        "breach_email_count": 1,
        "total_breach_count": 3
      },
      "items_report": {
        "owned_vault_count": 3,
        "owned_item_count": 148,
        "accessible_vault_count": 5,
        "accessible_item_count": 236
      },
      "monitor_report": {
        "user_id": 10482,
        "organization_id": 7201,
        "reused_passwords": 12,
        "inactive_2fa": 8,
        "excluded_items": 2,
        "weak_passwords": 5,
        "compromised_passwords": 3,
        "report_time": 1793520000,
        "client_version": "1.24.0"
      },
      "last_activity_time": 1793606400
    }
  ],
  "total_member_count": 1
}
```

If no member reports are returned, human output displays `No members found` and JSON output contains an empty `member_reports` array.

## Related commands

- **[user info](user.md#info)** - View your account information
- **[login](login.md)** - Authenticate to Proton Pass
- **[vault](vault.md)** - Manage vaults
