# SSH Agent

The Proton Pass CLI integrates nicely with any existing SSH workflows. It can either act as a SSH agent, or load your Pass-stored SSH keys into your already existing SSH agent. Let's see how to use both modes.

## Previous considerations

### SSH key management

Proton Pass CLI provides SSH key management capabilities. You can create new SSH keys or import existing ones directly into your vault. For detailed information on creating and importing SSH keys, see the [`item create ssh-key`](./item.md#create-ssh-key) documentation.

### Passphrase-protected SSH keys

Proton Pass allows you to generate new SSH keys, but it can also import and securely store your already-existing SSH keys.
If you are generating new SSH keys, there's no need to protect them with a passphrase, as they are already encrypted and securely stored within your Proton Pass vault.
However, if you are importing your already-existing SSH keys, probably they are using a passphrase for security reasons. If you want to import your passphrase-protected SSH keys, you can either:

- Create a copy of your unlocked private SSH key and import it into Proton Pass. For removing the passphrase of a SSH key you can use `ssh-keygen -p -f PATH_TO_YOUR_PRIVATE_KEY -N ""` (it will prompt your for your passphrase).
- Import your passphrase-protected private SSH key into Proton Pass and also create a custom field of type Hidden containing the passphrase. You can name it `Password` or `Passphrase`, but if you save it with any other name, Proton Pass CLI will try to use all the available `Hidden` custom fields to open it.

For more details on importing passphrase-protected keys, see the [SSH key import documentation](./item.md#create-ssh-key-import).

### SSH-Agent primer

An SSH agent is a small background program that safely holds your SSH keys in memory so you don't have to type your passphrase every time you connect to a server.

When you use `ssh` to connect somewhere, the agent's job is to:

1. Ask for your passphrase once to unlock your key in case the private key is locked.
2. Keep the unlocked key in memory (RAM).
3. Provide that key to SSH automatically whenever a server asks for authentication.

That means after you've "added" your key to the agent, you can `ssh` or `git pull` as many times as you want without needing to re-enter your password or specify which keys to use.

Chances are, if you are already using `ssh` for interacting with servers, you probably already have one running.

In case you don't, it's usually started by running:

```bash
eval $(ssh-agent)
```

> For macOS users, it's usually already started by default.

## SSH-Agent integration

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

After the tool loads the key you will see a summary like this one:

```bash
SSH Key Loading Summary:
  Successfully loaded: 0
  Already loaded (skipped): 3
  Total keys: 3

All keys were already present in the system SSH agent.
You can verify with: ssh-add -l
```

## Proton Pass CLI as your SSH agent

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

```text
SSH agent started successfully!
To use this agent, run:
  export SSH_AUTH_SOCK=/Users/youruser/.ssh/proton-pass-agent.sock

Keys will refresh automatically every 3600 seconds.

Press Ctrl+C to stop the agent.
```

When the SSH agent starts, it will create a unix socket in the default location, which is `$HOME/.ssh/proton-pass-agent.sock`. You can specify a custom location by passing the `--socket-path` flag:

```text
pass-cli ssh-agent start --socket-path MY_CUSTOM_SOCKET_PATH
```

In addition to that, the server periodically scans for new SSH keys that have been added to your monitored vaults. By default the check is done every hour, but you can configure it by specifying the `--refresh-interval` flag:

```bash
pass-cli ssh-agent start --refresh-interval 7200 # Every 2 hours, 7200 seconds
```

In order to use the ssh-agent, you need to run the `export` command that appears on screen, in the case of the example:

```text
export SSH_AUTH_SOCK=/Users/youruser/.ssh/proton-pass-agent.sock
```

### Creating new SSH key items automatically

!!! info "Feature available since version 1.3.0"

When using Proton Pass CLI as your SSH agent, you can enable automatic creation of new SSH key items. This feature is particularly useful if you want to import an existing SSH key using `ssh-add` and have it automatically stored in your Proton Pass vault.

To enable this feature, use the `--create-new-identities` flag followed by either a vault name or share ID:

```bash
pass-cli ssh-agent start --create-new-identities MySshKeysVault
pass-cli ssh-agent start --create-new-identities MY_SHARE_ID
```

When this option is enabled:

- Any SSH key you add using `ssh-add` will be automatically saved to the specified vault in Proton Pass (if it was not already loaded in the SSH agent)
- The new item will be created with a title based on the SSH key's comment (if available) or a shortened fingerprint
- You must have create permissions in the specified vault for this to work
- User-added identities are preserved when the agent periodically refreshes keys from Proton Pass

Example workflow:

```bash
# Start the agent with auto-creation enabled
pass-cli ssh-agent start --create-new-identities MySshKeysVault

# In another terminal, export the socket path
export SSH_AUTH_SOCK=$HOME/.ssh/proton-pass-agent.sock

# Add a new SSH key
ssh-add ~/.ssh/my_new_key

# The key is now automatically stored in your Proton Pass vault!
```

## Debugging SSH key items

The `ssh-agent debug` command helps you understand why your items are or aren't usable as SSH keys. This is useful when you have items in your vault but they're not being detected by the SSH agent.

To debug all items in a vault:

```bash
pass-cli ssh-agent debug --vault-name MySshKeysVault
pass-cli ssh-agent debug --share-id MY_SHARE_ID
```

To debug a specific item:

```bash
pass-cli ssh-agent debug --vault-name MySshKeysVault --item-title "my-github-key"
pass-cli ssh-agent debug --share-id MY_SHARE_ID --item-id ITEM_ID
```

The debug command will categorize each item as either valid or invalid, and provide clear reasons for why an item cannot be used:

**Valid SSH keys** will display:
- Item title
- Algorithm (RSA, Ed25519, ECDSA, DSA)
- Key size or curve name
- SHA256 fingerprint

**Invalid items** will show why they can't be used:
- Not an SSH key item (the item is a Login, Note, etc.)
- Item is trashed
- Invalid SSH private key format
- SSH key is encrypted but no passphrase found in custom fields
- Failed to decrypt SSH key (wrong passphrase)
- Malformed SSH key format

### Example output

```bash
SSH Agent Debug Report
Vault: Personal Vault (share-id-123)

✓ Valid SSH Keys (2):
  • my-github-key
    Algorithm: Ed25519
    Fingerprint: SHA256:abc123...

  • work-server
    Algorithm: RSA-4096
    Fingerprint: SHA256:def456...

✗ Invalid Items (2):
  • old-encrypted-key (SshKey)
    Reason: SSH key is encrypted but no passphrase found in custom fields

  • my-gmail-login (Login)
    Reason: Not an SSH key item (type: Login)

Summary:
  Valid SSH keys: 2
  Invalid items: 2
  Total items checked: 4
```

### JSON output

You can also get the output in JSON format for scripting purposes:

```bash
pass-cli ssh-agent debug --vault-name MySshKeysVault --output json
```

This will output structured JSON data with the same categorization and details.

## Troubleshooting

### `ssh-copy-id` fails due to having many ssh keys loaded and doesn't prompt for a password

A usual flow with a SSH agent is making sure we can log in with our SSH keys onto a new server, which is usually either done by:

1. The sysadmin adding our public SSH key for the desired remote user.
2. Ourselves performing a `ssh-copy-id` identifying with password for the first time in order to copy our SSH keys into the `authorized_keys` file.

For the second case, in case our SSH agent holds many SSH keys, when performing `ssh-copy-id` it will first try to authenticate using the SSH keys it holds.
If the remote server has a limit on the maximum number of authentication attempts, it's possible that we don't even get to the step where we are prompted for our password.
Take into account that, from the server's point of view, each SSH key the SSH Agent holds just looks like an authentication attempt, so it's like we are just trying to brute-force the login.

In order to force the `ssh-copy-id` command not to use the SSH keys of the agent, we can do it with the following command:

```bash
ssh-copy-id -o PreferredAuthentications=password -o PubkeyAuthentication=no [rest of arguments]
```

With these configurations, we explicitly say that we want to use password-based authentication and that public-key authentication should not be performed.

An example of a complete command could be something like:

```bash
ssh-copy-id -o PreferredAuthentications=password -o PubkeyAuthentication=no -p 2222 user@server
```

### Identities removed from the SSH agent keep reappearing

If you are using our SSH agent and you run `ssh-add -L` you will be able to see all the SSH keys that are loaded into the agent.
Our SSH agent supports **some of the `ssh-add` commands**. You can run commands like `ssh-add -D` to remove all the loaded SSH keys from the agent.
However, take into account that our SSH agent periodically refreshes the available SSH keys, so in case you run `ssh-add -L` from time to time, you would see them reappearing.
If that's a common flow you use, you should probably take a look at our [`ssh-agent load`](./ssh-agent.md#ssh-agent-integration) command to load your SSH keys stored in Proton Pass into your already-existing SSH agent.
