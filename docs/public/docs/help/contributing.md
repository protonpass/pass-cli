# Contributing

Thank you for your interest in contributing to Proton Pass CLI!

## Reporting Issues

If you find a bug or have a feature request, please check our [Troubleshooting guide](troubleshoot.md) first to see if it is a known issue or configuration problem.

## Development Workflow

If you are contributing code to the CLI, here are the recommended configuration options to set up your development environment.

### Debug Logging

Enable verbose logging to see what the CLI is doing under the hood. This is essential for debugging logic errors.

```bash
export PASS_LOG_LEVEL=debug
# or for maximum detail:
export PASS_LOG_LEVEL=trace
```

### Safe Key Storage for Development

When developing locally or in a container (like a devcontainer), you might not have access to the system keyring (macOS Keychain, Linux Secret Service). You can force the CLI to use a local file for key storage:

```bash
# WARNING: This stores the key in plain text in your session directory.
# Only use this for development, never in production.
export PROTON_PASS_KEY_PROVIDER=fs
```

See [Configuration](../get-started/configuration.md) for more details on storage backends.

### Testing

You can verify your session and connectivity with:

```bash
pass-cli test # Checks API connectivity
```

A good practice is to run a quick end-to-end test to ensure core functionality is working:

1.  `pass-cli vault create --name "DevTest"`
2.  `pass-cli item create login --vault-name "DevTest" --title "TestLogin" --username "user" --password "pass"`
3.  `pass-cli item list --vault-name "DevTest"`
4.  Clean up by deleting the vault when you are done.

For SSH Agent development, use the debug command to analyze why keys might be rejected:

```bash
pass-cli ssh-agent debug --vault-name "My Dev Vault"
```