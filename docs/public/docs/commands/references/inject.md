# `inject` Command

The `inject` command processes template files and replaces secret references with actual values. It uses handlebars-style syntax to identify and resolve `pass://` URIs.

## Synopsis

```bash
pass-cli inject [--in-file FILE] [--out-file FILE] [--force] [--file-mode MODE]
```

## Description

The `inject` command reads a template file (or stdin), finds all secret references wrapped in double braces `{{ pass://... }}`, resolves them from your Proton Pass vault, and outputs the processed template. This is useful for generating configuration files with secrets injected.

## How it works

1. **Read template**: Reads from `--in-file` or stdin
2. **Find references**: Uses regex to find `{{ pass://vault/item/field }}` patterns
3. **Resolve secrets**: Fetches each secret from Proton Pass
4. **Replace values**: Substitutes references with actual secret values
5. **Output**: Writes to `--out-file` or stdout
6. **Set permissions**: On Unix systems, sets file permissions when writing to a file

## Template syntax

The `inject` command uses handlebars-style syntax to identify secret references in templates. Secret references must be wrapped in double braces `{{ }}` to be processed.

For detailed information about secret references, see the [Secret References](secret-references.md) documentation.

### Basic syntax

Use double braces to mark secret references:

```text
{{ pass://vault/item/field }}
```

**Important**: 
- Only references wrapped in `{{ }}` are processed
- Plain `pass://` URIs in comments or elsewhere are ignored
- The double braces are required for the `inject` command (unlike `run` which processes bare `pass://` URIs)

### Reference format

Secret references follow this format:
```
{{ pass://<vault-identifier>/<item-identifier>/<field-name> }}
```

Where:
- **vault-identifier**: Vault Share ID or vault name
- **item-identifier**: Item ID or item title
- **field-name**: Field name (e.g., `password`, `username`, `api_key`)

See [Secret References](secret-references.md) for complete documentation on reference syntax, examples, and troubleshooting.

## Arguments

- `--in-file`, `-i`: Path to the template file. If not provided, reads from stdin.
- `--out-file`, `-o`: Path to write the processed output. If not provided, writes to stdout.
- `--force`, `-f`: Overwrite output file if it exists without prompting.
- `--file-mode`: Set file permissions for output file (Unix only, default: `0600`). Ignored if `--out-file` is not used.

## Mutually exclusive options

- Input source: Either `--in-file` or stdin (if `--in-file` is not provided). You cannot use both.
- Output destination: Either `--out-file` or stdout. If `--out-file` is provided, output goes to the file; otherwise, it goes to stdout.

## Examples

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

### Overwrite existing file

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

### Custom file permissions

```bash
pass-cli inject \
  --in-file template.txt \
  --out-file config.txt \
  --file-mode 0644
```

### Complete example

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

Process it:

```bash
pass-cli inject --in-file config.yaml.template --out-file config.yaml
```

The resulting `config.yaml` will have actual secret values instead of references.

## Related commands

- **[run](run.md)** - Execute commands with secrets injected from references
- **[secret-references](secret-references.md)** - Complete guide to secret reference syntax and usage

