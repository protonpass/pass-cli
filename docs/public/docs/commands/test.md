# `test` command

Test if an authenticated connection can be established with Proton Pass.

## Synopsis

```bash
pass-cli test
```

## Description

The `test` command verifies that your session is valid and that you can successfully communicate with the Proton Pass API by performing a ping operation. This is useful for troubleshooting connectivity issues and verifying your authentication status.

## How it works

The command performs a simple ping operation to the Proton Pass API. If successful, it confirms that:
- Your session is valid
- You can communicate with Proton Pass servers
- Your authentication tokens are working correctly

## Arguments

This command takes no arguments.

## Examples

### Basic connectivity test

```bash
pass-cli test
# Connection successful
```

### Using test in scripts

```bash
#!/bin/bash
if pass-cli test; then
    echo "Connection successful, proceeding with operations"
    pass-cli vault list
else
    echo "Connection failed, please login first"
    pass-cli login user@proton.me
fi
```

### Automated health check

```bash
#!/bin/bash
# Health check script
if ! pass-cli test > /dev/null 2>&1; then
    echo "Proton Pass CLI authentication failed"
    exit 1
fi
echo "Proton Pass CLI is ready"
```

## Common test results

### Successful test

When the test passes, you'll see output indicating successful connection to Proton Pass services.

### Authentication required

```bash
pass-cli test
# Error: This operation requires an authenticated client
```

This indicates you need to login first:

```bash
pass-cli login user@proton.me
pass-cli test
```

### Network connectivity issues

If you see network-related errors:
- Check your internet connection
- Verify firewall settings
- Confirm Proton services are accessible

### API service issues

If Proton Pass services are experiencing issues:
- Check Proton's status page
- Try again after some time
- Contact Proton support if issues persist

## Use cases

### Development workflow

```bash
# Verify setup before starting work
pass-cli test && echo "Ready to work"
```

### Automated monitoring

```bash
# Cron job to monitor CLI health
0 * * * * /usr/local/bin/pass-cli test || echo "Pass CLI authentication expired" | mail -s "Alert" admin@company.com
```

### Troubleshooting

```bash
# Step-by-step troubleshooting
echo "Testing connection..."
pass-cli test

echo "Checking session info..."
pass-cli info

echo "Listing accessible vaults..."
pass-cli vault list
```

## Troubleshooting test failures

### Authentication issues

1. **No session**: Login first with `pass-cli login`
2. **Expired session**: Re-authenticate with `pass-cli login`
3. **Invalid credentials**: Logout and login again

### Network issues

1. **Connectivity**: Check internet connection
2. **Firewall**: Ensure Proton Pass API endpoints are accessible
3. **DNS**: Verify DNS resolution for Proton domains

### Service issues

1. **API status**: Check if Proton Pass services are operational
2. **Maintenance**: Services may be temporarily unavailable
3. **Rate limiting**: You may have exceeded API rate limits

## Related commands

- **[login](login.md)** - Authenticate with Proton Pass
- **[logout](logout.md)** - End current session
- **[info](info.md)** - Display detailed session information
