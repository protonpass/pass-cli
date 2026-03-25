# `pat` / `personal-access-token` command

Create and manage personal access tokens.

## Synopsis

The namespace can be specified as either personal-access-token or its shorthand pat; both behave identically.

```bash
pass-cli pat <COMMAND>

# Or

pass-cli personal-access-token <COMMAND>
```

## Description

Personal access tokens let you authenticate with Proton Pass without using your full account credentials. Each token can be scoped to specific vaults or individual items, and you control the permission level - which makes them well suited for CI pipelines, automated scripts, or any environment where you don't want to hand over full account access.

Tokens have a mandatory expiration date, so they automatically stop working after a set period.

## Subcommands

| Command | Description |
|---|---|
| `create` | Create a new personal access token |
| `list` | List all personal access tokens |
| `delete` | Delete a personal access token |
| `renew` | Renew a personal access token with a new expiration |
| `access grant` | Grant a token access to a vault or item |
| `access revoke` | Revoke a token's access to a vault or item |
| `access list-access` | List what a token has access to |

---

## `pat create`

```bash
pass-cli pat create --name <NAME> --expiration <EXPIRATION> [--output human|json]
```

Creates a new personal access token. The token is printed immediately after creation. **This is the only time the full token value is shown, so make sure to save it somewhere safe**.

### Arguments

| Flag | Required | Description                                                                            |
|---|---|----------------------------------------------------------------------------------------|
| `--name` | Yes | A descriptive name for the token                                                       |
| `--expiration` | Yes | How long until the token expires: `1d`, `1w`, `1m`, `3m`, `6m`, `1y`                   |
| `--output` | No | Output format: `human` (default, unless you have defined it in the `settings`) or `json` |

### Example

```bash
pass-cli pat create --name "deploy-bot" --expiration 3m
# PROTON_PASS_PERSONAL_ACCESS_TOKEN=pst_xxxx...xxxx::TOKENKEY
```

The output is ready to use as an environment variable. After creating the token, grant it access to the vaults or items it needs (see `pat access grant` below).

---

## `pat list`

```bash
pass-cli pat list [--output human|json]
```

Lists all personal access tokens on your account, along with their IDs and expiration dates.

```bash
pass-cli pat list
# - [abc123]: deploy-bot (expires: 2025-06-01)
# - [def456]: staging-reader (expires: 2025-07-15)
```

---

## `pat delete`

```bash
pass-cli pat delete --personal-access-token-id <ID>
```

Permanently deletes a personal access token. Any system using that token will immediately lose access.

```bash
pass-cli pat delete --personal-access-token-id abc123
```

---

## `pat renew`

```bash
pass-cli pat renew (--personal-access-token-id <ID> | --personal-access-token-name <NAME>) \
    --expiration <EXPIRATION> [--output human|json]
```

Renews a token with a new expiration date, starting from now. A renewed token outputs a new token string - treat it the same as a freshly created token and update your secrets accordingly. Any access you had granted to that token will not be affected, so the token will continue to have access to it.

### Arguments

| Flag                                         | Required | Description |
|----------------------------------------------|---|---|
| `--personal-access-token-id` / `--pat-id`    | One of these | Token ID to renew |
| `--personal-access-token-name` / `--pat-name` | One of these | Token name to renew |
| `--expiration`                               | Yes | New expiration: `1d`, `1w`, `1m`, `3m`, `6m`, `1y` |
| `--output`                                   | No | Output format: `human` or `json` |

```bash
pass-cli pat renew --personal-access-token-name "deploy-bot" --expiration 3m
# PROTON_PASS_PERSONAL_ACCESS_TOKEN=pst_xxxx...xxxx::TOKENKEY
```

---

## `pat access grant`

```bash
pass-cli pat access grant \
    (--personal-access-token-id <ID> | --personal-access-token-name <NAME>) \
    (--share-id <SHARE_ID> | --vault-name <VAULT_NAME>) \
    [--item-id <ITEM_ID> | --item-title <ITEM_TITLE>] \
    [--role viewer|editor|manager]
```

Grants a token access to a vault or a specific item within a vault. By default, access is granted with the `viewer` role.

### Arguments

| Flag | Required | Description |
|---|---|---|
| `--personal-access-token-id` / `--pat-id` | One of these | Token ID |
| `--personal-access-token-name` / `--pat-name` | One of these | Token name |
| `--share-id` | One of these | Vault share ID |
| `--vault-name` | One of these | Vault name |
| `--item-id` | No | Restrict access to a specific item by ID |
| `--item-title` | No | Restrict access to a specific item by title |
| `--role` | No | Permission level: `viewer` (default), `editor`, or `manager` |

If neither `--item-id` nor `--item-title` is provided, access is granted to the entire vault.

### Examples

Grant read-only access to a whole vault:

```bash
pass-cli pat access grant --pat-name "deploy-bot" --vault-name "Production" --role viewer
```

Grant access to a single item only:

```bash
pass-cli pat access grant --pat-name "deploy-bot" --vault-name "Production" --item-title "DB password"
```

---

## `pat access revoke`

```bash
pass-cli pat access revoke \
    (--personal-access-token-id <ID> | --personal-access-token-name <NAME>) \
    --share-id <SHARE_ID>
```

Revokes a token's access to a specific vault.

```bash
pass-cli pat access revoke --pat-name "deploy-bot" --share-id <SHARE_ID>
```

---

## `pat access list-access`

```bash
pass-cli pat access list-access \
    (--personal-access-token-id <ID> | --personal-access-token-name <NAME>) \
    [--output human|json]
```

Shows all vaults and items a token currently has access to, along with the role and expiration time for each grant.

```bash
pass-cli pat access list-access --pat-name "deploy-bot"
# Personal access token access grants:
#
# - [share_abc] Production | Type=Vault | Role=Viewer | Expires: 2025-06-01 00:00 (UTC)
# - [share_abc] DB password | Type=Item | Role=Viewer | Expires: 2025-06-01 00:00 (UTC)
```

---

## Typical workflow

Here's a full setup from scratch:

```bash
# 1. Create a token valid for 3 months
pass-cli pat create --name "ci-runner" --expiration 3m
# PROTON_PASS_PERSONAL_ACCESS_TOKEN=pst_xxxx...xxxx::TOKENKEY

# 2. Grant it read-only access to the vault it needs
pass-cli pat access grant --pat-name "ci-runner" --vault-name "CI Secrets" --role viewer

# 3. Store the token in your CI secrets and use it to log in
PROTON_PASS_PERSONAL_ACCESS_TOKEN=pst_xxxx...xxxx::TOKENKEY pass-cli login
```

## Related commands

- [`login`](login.md) - authenticate using a personal access token
- [`info`](info.md) - check the current session (shows "Personal Access Token: \<name\>" for PAT sessions)
- [`logout`](logout.md) - end the current session
