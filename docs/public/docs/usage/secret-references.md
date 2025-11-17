# Secret References

The CLI uses a URL-like syntax to reference secrets stored in Pass:

```text
pass://<vault-name-or-id>/<item-name-or-id>/<field-name>
```

## Examples

```text
pass://Work/GitHub/password
pass://Personal/Email Login/username
pass://AbCdEf123456/XyZ789/password
pass://My Vault/My Item/My Custom Field
```

## Notes

- Vault and item can be referenced by name or ID
- Names with spaces are supported
- Field name must match exactly (case-sensitive)
- Common fields: `username`, `password`, `email`, `url`, `note`

Secret references are used with the [`run`](run.md) and [`inject`](inject.md) commands to inject secrets into your applications.

