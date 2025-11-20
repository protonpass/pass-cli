# `run` command

Execute commands with secrets from Proton Pass injected as environment variables.

## Synopsis

```bash
pass-cli run [--env-file FILE]... [--no-masking] -- COMMAND [ARGS...]
```

## Description

The `run` command executes external commands or scripts while providing secrets from Proton Pass as environment variables. It scans environment variables (from the current process and `.env` files) for `pass://` URI references, resolves them from your Proton Pass vault, and injects the resolved values before running your command.

## How it works

1. **Collect environment variables**: Gathers variables from:
   - Current process environment
   - `.env` files specified with `--env-file` (processed in order, later files override earlier ones)
2. **Find secret references**: Scans all environment variable values for `pass://` URIs
3. **Resolve secrets**: Fetches each referenced secret from Proton Pass
4. **Replace values**: Substitutes `pass://` URIs with actual secret values
5. **Mask output** (default): Masks secret values in stdout/stderr to prevent accidental exposure
6. **Execute command**: Runs your command with the resolved environment variables
7. **Forward I/O**: Properly forwards stdin, stdout, and stderr to/from the child process
8. **Handle signals**: Forwards SIGTERM/SIGINT to the child process for graceful shutdown

## Arguments

- `--env-file FILE`: Load environment variables from a dotenv file. Can be specified multiple times. Files are processed in order, with later files overriding earlier ones. Each line should be in `KEY=VALUE` format.
- `--no-masking`: Disable automatic masking of secrets in stdout and stderr output. By default, secret values are replaced with `<concealed by Proton Pass>` in output.
- `COMMAND [ARGS...]`: The command and its arguments to execute. Must come after `--` separator. Required.

## Mutually exclusive options

There are no mutually exclusive options. All options can be used together:

- Multiple `--env-file` options can be specified
- `--no-masking` can be combined with `--env-file` options

## Secret reference syntax

The `run` command resolves item references in environment variables. Secret references use the `pass://` URI syntax to point to secrets stored in your Proton Pass vaults.

For detailed information about item references, see the [item references](item-references.md) documentation.

### Basic syntax

Environment variables can reference secrets using `pass://` URIs:

```bash
VARIABLE_NAME="pass://vault-name/item-name/field-name"
```

### Multiple secrets in a single value

You can include multiple secret references or mix them with other text:

```bash
DATABASE_URL="postgresql://user:pass://vault/db/password@localhost/db"
API_ENDPOINT="https://api.example.com?key=pass://vault/api/key"
```

**Important**:

- The `pass://` URI must appear directly in the environment variable value
- It will be replaced with the actual secret value before the command runs
- Multiple references in the same value are all resolved
- References can be mixed with plain text

## Examples

### Basic usage

```bash
# Set environment variable with secret reference
export DB_PASSWORD='pass://Production/Database/password'

# Run application with injected secret
pass-cli run -- ./my-app
```

The application sees `DB_PASSWORD` with the actual password value.

### Using .env files

Create a `.env` file:

```bash
DB_HOST=localhost
DB_PORT=5432
DB_USERNAME=admin
DB_PASSWORD=pass://Production/Database/password
API_KEY=pass://Work/External API/api_key
```

Run with the env file:

```bash
pass-cli run --env-file .env -- ./my-app
```

### Multiple env files

```bash
pass-cli run \
  --env-file base.env \
  --env-file secrets.env \
  --env-file local.env \
  -- ./my-app
```

Files are processed in order, so `local.env` overrides `secrets.env`, which overrides `base.env`.

### Secret masking

By default, secrets are masked in output:

```bash
pass-cli run -- ./my-app
# If the app logs: API_KEY: sk_live_abc123
# Output shows: API_KEY: <concealed by Proton Pass>
```

Disable masking:

```bash
pass-cli run --no-masking -- ./my-app
```

### Running with arguments

```bash
pass-cli run -- ./my-app --config production --verbose
```

### Interactive programs

The command supports stdin/stdout/stderr forwarding:

```bash
pass-cli run -- python
```

### Complex example

```bash
#!/bin/bash
# Set some environment variables
export NODE_ENV=production
export LOG_LEVEL=info

# Run with secrets from .env files
pass-cli run \
  --env-file .env.production \
  --env-file .env.secrets \
  -- node server.js --port 3000
```

### CI/CD integration

```bash
#!/bin/bash
# CI/CD deployment script

# Load production secrets
pass-cli run \
  --env-file .env.production \
  -- ./deploy.sh
```

## Security considerations

### Secret masking

- **Default behavior**: Secrets are automatically masked in stdout/stderr
- **Disable carefully**: Only use `--no-masking` when necessary and safe
- **Log files**: Be aware that unmasked secrets may appear in log files

### Environment variable handling

- **Process isolation**: Secrets are only available to the child process
- **No persistence**: Secrets are not stored in your shell environment
- **Override order**: Later `.env` files override earlier ones

### Best practices

- Use `.env` files for secret references, not direct environment variables
- Keep `.env` files out of version control
- Use different `.env` files for different environments
- Review output carefully when using `--no-masking`

## Troubleshooting

### Secret not found

If a `pass://` URI cannot be resolved:

- Verify the vault, item, and field exist
- Check you have access to the vault
- Ensure the URI format is correct

### Command not found

If your command isn't found:

- Ensure the command is in your PATH
- Use full path: `pass-cli run -- /usr/bin/my-app`
- Check file permissions if using a script

### Environment variable not set

If your application doesn't see an environment variable:

- Verify the variable is in your `.env` file or exported
- Check for typos in variable names
- Ensure the `pass://` URI is correctly formatted
