# SSH Agent

The Proton Pass CLI integrates nicely with any existing SSH workflows. It can either act as a SSH agent, or load your Pass-stored SSH keys into your already existing SSH agent. Let's see how to use both modes.

## Previous considerations

### Passphrase-protected SSH keys

Proton Pass allows you to generate new SSH keys, but it can also import and securely store your already-existing SSH keys.
If you are generating new SSH keys, there's no need to protect them with a passphrase, as they are already encrypted and securely stored within your Proton Pass vault.
However, if you are importing your already-existing SSH keys, probably they are using a passphrase for security reasons. If you want to import your passphrase-protected SSH keys, you can either:

- Create a copy of your unlocked private SSH key and import it into Proton Pass. For removing the passphrase of a SSH key you can use `ssh-keygen -p -f PATH_TO_YOUR_PRIVATE_KEY -N ""` (it will prompt your for your passphrase).
- Import your passphrase-protected private SSH key into Proton Pass and also create a custom field of type Hidden containing the passphrase. You can name it `Password` or `Passphrase`, but if you save it with any other name, Proton Pass CLI will try to use all the available `Hidden` custom fields to open it.

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

