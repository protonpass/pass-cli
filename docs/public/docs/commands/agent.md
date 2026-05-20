# Agent

Agents are a way to give an AI agent or automated process scoped, audited access to your Proton Pass secrets.
Each agent is a personal access token with a dedicated flag that enables access logging, so you can see which items were read, when, and for what stated reason.

## How it works

Under the hood an agent is a [personal access token](./personal-access-token.md) with a special consideration.
When an agent performs certain operations, such as viewing or interacting with an item, it must supply a reason via `PROTON_PASS_AGENT_REASON`. Then, that reason is encrypted and stored in an audit log that you can inspect later with `agent monitor`. This makes agents more suitable than plain PATs when you want visibility into what an automated process is actually doing with your secrets.

The following commands require a reason specified when being run by an agent:
* `item view`
* `item create` (any of its variants)
* `item update`
* `item trash`
* `item untrash`
* `item move`
* `vault update`

Authentication works the same way as a plain PAT: set `PROTON_PASS_PERSONAL_ACCESS_TOKEN` and call `pass-cli login`. For more details you can look at the [Login command](./login.md).

## Providing a reason

Before running any of the audited commands, the agent must set the `PROTON_PASS_AGENT_REASON` environment variable. It must be non-empty and at most 300 characters. It is stored end-to-end-encrypted alongside the log entry so the agent owner can review what the agent read and why.

```bash
PROTON_PASS_AGENT_REASON="Running nightly backup" pass-cli item view \
    --vault-name "Production" \
    --item-name "DB password" \
    --field password
```

## Typical workflow

User:

```bash
# 1. Log in with your account
pass-cli login

# 2. Create an agent valid for 1 month, with access to a specific vault
pass-cli agent create my-agent --expiration 1m --vault "Production"
# Output (JSON) which you can send to your AI agent:
# {
#   "token": "PROTON_PASS_PERSONAL_ACCESS_TOKEN=pst_xxxx...xxxx::TOKENKEY",
#   "instruction": "..."
# }

# The AI agent performs some operations...

# 3. Inspect the audit log as the account owner
pass-cli agent monitor my-agent
```

AI agent:

```bash
# The user will send the output to the agent and it should log in like this:

# 1. Optionally use a separate session directory to avoid overwriting an existing session
export PROTON_PASS_SESSION_DIR="/tmp/pass-agent-my-agent"

# 2. Check whether an existing session is already valid before logging in or log in
pass-cli info 2>/dev/null || PROTON_PASS_PERSONAL_ACCESS_TOKEN=pst_xxxx...xxxx::TOKENKEY pass-cli login

# 3. Read a secret, passing the reason inline
PROTON_PASS_AGENT_REASON="Running nightly backup" pass-cli agent item view \
    --vault-name "Production" \
    --item-name "DB password" \
    --field password
```

---

## `agent create`

```bash
pass-cli agent create <NAME> --expiration <EXPIRATION> [--vault <VAULT_NAME>] [--vault <VAULT_NAME>]...
```

Creates a new agent. The output is always JSON and contains the token string and a usage hint. **This is the only time the token value is shown**, save it somewhere safe before closing the terminal.

| Argument | Required | Description |
|---|---|---|
| `<NAME>` | Yes | A name for the agent |
| `--expiration` | Yes | Token lifetime: `1d`, `1w`, `1m`, `3m`, `6m`, `1y` |
| `--vault` | No | Vault name to grant access to. Can be repeated to grant access to multiple vaults |

If you skip `--vault` you can grant access later with `agent access grant`. You may specify `--vault <VAULT_NAME>` many times to grant access to resources in one go.

```bash
pass-cli agent create my-agent --expiration 3m --vault "Production" --vault "Staging"
# {
#   "token": "PROTON_PASS_PERSONAL_ACCESS_TOKEN=pst_xxxx...xxxx::TOKENKEY",
#   "instruction": "..."
# }
```

---

## `agent list`

```bash
pass-cli agent list [--output human|json]
```

Lists all agents on your account with their IDs and expiration dates.

```bash
pass-cli agent list
# - [pat_abc123]: my-agent (expires: 2026-07-20)
```

---

## `agent delete`

```bash
pass-cli agent delete <NAME>
```

Permanently deletes an agent. Any process using that agent's token will immediately lose access.

```bash
pass-cli agent delete my-agent
```

---

## `agent renew`

```bash
pass-cli agent renew <NAME> --expiration <EXPIRATION> [--output human|json]
```

Issues a new token for the agent with a new expiration date. The old token stops working immediately. Access grants are preserved. Update the token value in your agent's environment after renewing.

| Argument | Required | Description |
|---|---|---|
| `<NAME>` | Yes | Agent name |
| `--expiration` | Yes | New lifetime: `1d`, `1w`, `1m`, `3m`, `6m`, `1y` |
| `--output` | No | `json` (default) prints the full JSON payload; `human` prints only the token string |

```bash
pass-cli agent renew my-agent --expiration 3m
# {
#   "token": "PROTON_PASS_PERSONAL_ACCESS_TOKEN=pst_xxxx...xxxx::NEWTOKEN",
#   "instruction": "..."
# }
```

---

## `agent access grant`

```bash
pass-cli agent access grant <NAME> \
    (--share-id <SHARE_ID> | --vault-name <VAULT_NAME>) \
    [--item-id <ITEM_ID> | --item-title <ITEM_TITLE>] \
    [--role viewer|editor|manager]
```

Grants an agent access to a vault or to a single item within a vault. Defaults to `viewer` role.

| Flag | Required | Description |
|---|---|---|
| `<NAME>` | Yes | Agent name |
| `--share-id` | One of these | Vault share ID |
| `--vault-name` | One of these | Vault name |
| `--item-id` | No | Restrict access to a specific item by ID |
| `--item-title` | No | Restrict access to a specific item by title |
| `--role` | No | `viewer` (default), `editor`, or `manager` |

When neither `--item-id` nor `--item-title` is given, the agent gets access to the whole vault.

```bash
# Grant read access to a whole vault
pass-cli agent access grant my-agent --vault-name "Production" --role viewer

# Grant access to a single item only
pass-cli agent access grant my-agent --vault-name "Production" --item-title "DB password"
```

---

## `agent access revoke`

```bash
pass-cli agent access revoke <NAME> --share-id <SHARE_ID>
```

Revokes an agent's access to a vault.

```bash
pass-cli agent access revoke my-agent --share-id <SHARE_ID>
```

---

## `agent monitor`

```bash
pass-cli agent monitor [<NAME>] [--limit <N>] [--output human|json]
```

Shows the audit log for an agent. Each entry records which item was accessed, from which vault, the action taken, and the reason provided at access time.

When you are logged in as your user account, `<NAME>` is required. When you are logged in as the agent itself (via `PROTON_PASS_PERSONAL_ACCESS_TOKEN`), `<NAME>` can be omitted.

| Argument | Required | Description |
|---|---|---|
| `<NAME>` | When logged in as user | Agent name |
| `--limit` | No | Maximum number of records to return (default: 100) |
| `--output` | No | `human` (default) or `json` |

```bash
pass-cli agent monitor my-agent
# [record_001] action=ItemRead vault="Production" item="DB password" reason="Running nightly backup" (object_id=item_xyz)
```

---

## `agent instructions`

```bash
pass-cli agent instructions
```

Prints the agent usage instructions to stdout as a Markdown document. Redirect the output to a file to create a reference document or a skill file for your AI tooling.

```bash
pass-cli agent instructions > agent-instructions.md
```

---

## Related commands

- [`personal-access-token`](./personal-access-token.md) - create and manage PATs directly
- [`login`](./login.md) - authenticate using a token
- [`item`](./item.md) - item management and the `pass://` URI format
