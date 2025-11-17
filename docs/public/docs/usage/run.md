# The `run` Command

The `run` command executes a program with secrets injected as environment variables. It searches environment variables for secret references and resolves them before running the command.

## Basic usage

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

## Using .env files

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

## Secret masking

By default, the `run` command masks secrets in stdout/stderr:

```bash
pass-cli run -- ./my-app
```

If the application logs `API_KEY: sk_live_abc123`, the output shows:

```text
API_KEY: <concealed by Proton Pass>
```

Disable masking:

```bash
pass-cli run --no-masking -- ./my-app
```

## Running with arguments

Pass arguments to your application:

```bash
pass-cli run -- ./my-app --config production --verbose
```

## Interactive programs

The `run` command supports stdin/stdout/stderr forwarding, so interactive programs work normally:

```bash
pass-cli run -- python
```

## Signal handling

`Ctrl+C` (SIGTERM) is properly forwarded to the child process. The CLI waits for graceful shutdown before sending SIGKILL if needed.

