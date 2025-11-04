# Data operations and workflows

This document provides a practical guide to using the Proton Pass CLI for managing vaults, items, and secrets in your applications.

## Overview

The CLI operates on a hierarchical data model:
```
Account
└── Share (representing either Vaults or Items shared with you)
    └── Items (logins, notes, passwords, etc.)
        └── Fields (username, password, custom fields, etc.)
```

## Working with vaults

Vaults are containers for items. Each vault has a unique ID and is accessed through a unique share ID.

You can think of a Share as the relationship between a User and a Resource.
Many users can access the same vault (with the same VaultID), but each of them will do it based on their own Share.
Resources can be either Vaults or Items (a user can share with another user access to an entire Vault, or just to a specific Item).

### List all vaults

```bash
pass-cli vault list
```

Output shows:
- Vault name
- Share ID (needed for other operations)
- Item count
- Owner information

### Create a vault

```bash
pass-cli vault create "Work Passwords"
```

This creates a new vault and returns its share ID:
```
Created vault with id: AbCdEf123456
```

**Note:** A default vault named "Personal" is automatically created on first login if no vaults exist.

### Get share list

```bash
pass-cli share list
```

This shows detailed information about all shares you have access to, both for Vaults and also Items shared with you.

## Working with items

Items are the actual credentials and data stored in vaults.

### List items in a vault

For listing items in a vault, you can use the Share if of the vault.

```bash
pass-cli item list --share-id AbCdEf123456
```

Or you can specify the vault name

```bash
pass-cli item list "Personal"
```

### Create a new item

In order to see which item types you can create, you can run

```bash
pass-cli item create --help
```

Then, after you know which type of item you want to create, you can run `--help` again to see which options are available. Take into account that not all item types support the same creation options.
As an example, let's see a few ways for creating a new login item:

**Basic creation:**
```bash
pass-cli item create login \
  --share-id AbCdEf123456 \
  --title "GitHub Account" \
  --username "octocat" \
  --password "secret123" \
  --url "https://github.com"
```

**Generate a random password:**
```bash
pass-cli item create login \
  --share-id AbCdEf123456 \
  --title "New Account" \
  --username "user@example.com" \
  --generate-password
```

**Generate password with custom settings:**
```bash
pass-cli item create login \
  --share-id AbCdEf123456 \
  --title "New Account" \
  --username "user@example.com" \
  --generate-password=32,uppercase,symbols
```

Format: `length,uppercase,symbols` (numbers are always included)

**Generate a passphrase:**

```bash
pass-cli item create login \
  --share-id AbCdEf123456 \
  --title "New Account" \
  --username "user@example.com" \
  --generate-passphrase=5
```

This generates a passphrase with 5 words.

### Create from template

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

### View an item

The CLI can print the full details of an item by specifying both the Share id and the Item id

```bash
pass-cli item view --share-id AbCdEf123456 --item-id XyZ789
```

And also by specifying the path in a URI format:

```bash
pass-cli item view "pass://Personal/TestItem"
```

### View an item field

The CLI can print a single field of an item by specifying the Share id, the Item id and the field name

```bash
pass-cli item view --share-id AbCdEf123456 --item-id XyZ789 --field password
```

And also by specifying the path in a URI format:

```bash
pass-cli item view "pass://Personal/TestItem/password"
```

### Delete an item

```bash
pass-cli item delete --share-id AbCdEf123456 --item-id XyZ789
```

## Secret references

The CLI uses a URL-like syntax to reference secrets stored in Pass:

```
pass://<vault-name-or-id>/<item-name-or-id>/<field-name>
```

Examples:
```
pass://Work/GitHub/password
pass://Personal/Email Login/username
pass://AbCdEf123456/XyZ789/password
pass://My Vault/My Item/My Custom Field
```

**Notes:**
- Vault and item can be referenced by name or ID
- Names with spaces are supported
- Field name must match exactly (case-sensitive)
- Common fields: `username`, `password`, `email`, `url`, `note`

## The `run` command

The `run` command executes a program with secrets injected as environment variables. It searches environment variables for secret references and resolves them before running the command.

### Basic usage

```bash
# Set environment variables with secret references
export DB_PASSWORD='pass://Production/Database/password'
export API_KEY='pass://Work/External API/api_key'

# Run a command
pass-cli run -- ./my-app
```

The application sees:

```bash
DB_PASSWORD='actual-secret-value'
API_KEY='actual-api-key-value'
```

### Using .env files

Create a `.env` file:

```bash
cat > .env << EOF
DB_HOST=localhost
DB_PORT=5432
DB_USERNAME=admin
DB_PASSWORD=pass://Production/Database/password
API_KEY=pass://Work/External API/api_key
API_SECRET=pass://Work/External API/secret
EOF
```

Run with the env file:
```bash
pass-cli run --env-file .env -- ./my-app
```

Multiple env files are supported:

```bash
pass-cli run \
  --env-file base.env \
  --env-file secrets.env \
  --env-file local.env \
  -- ./my-app
```

### Secret masking

By default, the `run` command masks secrets in stdout/stderr:

```bash
pass-cli run -- ./my-app
```

If the application logs `API_KEY: sk_live_abc123`, the output shows:

```
API_KEY: <concealed by Proton Pass>
```

Disable masking:

```bash
pass-cli run --no-masking -- ./my-app
```

### Running with arguments

Pass arguments to your application:
```bash
pass-cli run -- ./my-app --config production --verbose
```

### Interactive programs

The `run` command supports stdin/stdout/stderr forwarding, so interactive programs work normally:

```bash
pass-cli run -- python
```

### Signal handling

`Ctrl+C` (SIGTERM) is properly forwarded to the child process. The CLI waits for graceful shutdown before sending SIGKILL if needed.

## The `inject` command

The `inject` command processes template files and replaces secret references with actual values. It uses handlebars-style syntax.

### Template syntax

Use double braces to mark secret references:
```
{{ pass://vault/item/field }}
```

Create a template file:
```yaml
# config.yaml.template
database:
  host: localhost
  port: 5432
  username: {{ pass://Production/Database/username }}
  password: {{ pass://Production/Database/password }}

api:
  key: {{ pass://Work/API Keys/api_key }}
  secret: {{ pass://Work/API Keys/secret }}

# This comment with pass://fake/uri is ignored
# Only {{ }} wrapped references are processed
```

### Inject to stdout

```bash
pass-cli inject --in-file config.yaml.template
```

This prints the processed template to stdout.

### Inject to file

```bash
pass-cli inject \
  --in-file config.yaml.template \
  --out-file config.yaml
```

If the output file exists, add `--force`:

```bash
pass-cli inject \
  --in-file config.yaml.template \
  --out-file config.yaml \
  --force
```

### Read from stdin

```bash
cat template.txt | pass-cli inject
```

Or with heredoc:
```bash
pass-cli inject << EOF
{
  "database": {
    "password": "{{ pass://Production/Database/password }}"
  }
}
EOF
```

## SSH integration

The Proton Pass CLI integrates nicely with any existing SSH workflows. It can either act as a SSH agent, or load your Pass-stored SSH keys into your already existing SSH agent. Let's see how to use both modes.

### Previous considerations

Proton Pass allows you to generate new SSH keys, but it can also import and securely store your already-existing SSH keys.
If you are generating new SSH keys, there's no need to protect them with a passphrase, as they are already encrypted and securely stored within your Proton Pass vault.
However, if you are importing your already-existing SSH keys, probably they are using a passphrase for security reasons. If you want to import your passphrase-protected SSH keys, you can either:

* Create a copy of your unlocked private SSH key and import it into Proton Pass. For removing the passphrase of a SSH key you can use `ssh-keygen -p -f PATH_TO_YOUR_PRIVATE_KEY -N ""` (it will prompt your for your passphrase).
* Import your passphrase-protected private SSH key into Proton Pass and also create a custom field of type Hidden containing the passphrase. You can name it `Password` or `Passphrase`, but if you save it with any other name, Proton Pass CLI will try to use all the available `Hidden` custom fields to open it.
  
An SSH agent is a small background program that safely holds your SSH keys in memory so you don’t have to type your passphrase every time you connect to a server.

When you use `ssh` to connect somewhere, the agent's job is to:

1. Ask for your passphrase once to unlock your key in case the private key is locked.
2. Keep the unlocked key in memory (RAM).
3. Provide that key to SSH automatically whenever a server asks for authentication.

That means after you’ve "added" your key to the agent, you can `ssh` or `git pull` as many times as you want without needing to re-enter your password or specify which keys to use.

Chances are, if you are already using `ssh` for interacting with servers, you probably already have one running.

In case you don't, it's usually started by running:

```bash
eval $(ssh-agent)
```

> For macOS users, it's usually already started by default.

### SSH-Agent integration

Proton Pass CLI can load your SSH keys into your existing SSH agent. 

For doing so, make sure the `SSH_AUTH_SOCK` environment variable is defined. If it is, you can load your SSH keys into the agent by running the following command:

```bash
pass-cli ssh-agent load
```

It will then proceed to scan your vaults looking for items of type "SSH key", try to open them in case they are locked, check if they are already loaded into the SSH agent, and in case they aren't, load them so they can be used.

You can also restrict which vaults to look for by using the `--share-id` or `--vault-name` parameters:

```bash
pass-cli ssh-agent load --share-id MY_SHARE_ID
pass-cli ssh-agent load --vault-name MySshKeysVault
```

After the tool loads the key you will see a summary like this one

```bash
SSH Key Loading Summary:
  Successfully loaded: 0
  Already loaded (skipped): 3
  Total keys: 3

All keys were already present in the system SSH agent.
You can verify with: ssh-add -l
```

### Proton Pass CLI as your SSH agent

Proton Pass CLI can also work as a SSH agent itself. For doing so, you can start it by running the following command:

```bash
pass-cli ssh-agent start 
```

You can also restrict which vaults to look for by using the `--share-id` or `--vault-name` parameters:

```bash
pass-cli ssh-agent start --share-id MY_SHARE_ID
pass-cli ssh-agent start --vault-name MySshKeysVault
```

After it's started, you will see an output like this one:

```
SSH agent started successfully!
To use this agent, run:
  export SSH_AUTH_SOCK=/Users/youruser/.ssh/proton-pass-agent.sock

Keys will refresh automatically every 3600 seconds.

Press Ctrl+C to stop the agent.
```

When the SSH agent starts, it will create a unix socket in the default location, which is `$HOME/.ssh/proton-pass-agent.sock`. You can specify a custom location by passing the `--socket-path` flag:

```
pass-cli ssh-agent start --socket-path MY_CUSTOM_SOCKET_PATH
```

In addition to that, the server periodically scans for new SSH keys that have been added to your monitored vaults. By default the check is done every hour, but you can configure it by specifying the `--refresh-interval` flag:

```bash
pass-cli ssh-agent start --refresh-interval 7200 # Every 2 hours, 7200 seconds
```

In order to use the ssh-agent, you need to run the `export` command that appears on screen, in the case of the example:

```
export SSH_AUTH_SOCK=/Users/youruser/.ssh/proton-pass-agent.sock
```
