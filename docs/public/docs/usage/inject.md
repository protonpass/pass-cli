# The `inject` Command

The `inject` command processes template files and replaces secret references with actual values. It uses handlebars-style syntax.

## Template syntax

Use double braces to mark secret references:

```text
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

## Inject to stdout

```bash
pass-cli inject --in-file config.yaml.template
```

This prints the processed template to stdout.

## Inject to file

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

## Read from stdin

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

