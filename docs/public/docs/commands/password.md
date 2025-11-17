# `password` command

Generate and analyze passwords.

## Synopsis

```bash
pass-cli password <SUBCOMMAND>
```

## Description

The `password` command provides utilities for generating secure passwords and passphrases, as well as analyzing password strength. These operations don't require vault access and can be used independently.

## Subcommands

### generate

Generate secure passwords or passphrases.

```bash
pass-cli password generate <TYPE>
```

**Types:**
- `random` - Generate a random password with customizable settings
- `memorable` - Generate a memorable passphrase using words

#### generate random

Generate a random password.

```bash
pass-cli password generate random [OPTIONS]
```

**Options:**
- `--length LENGTH` - Password length (default varies)
- `--numbers BOOL` - Include numbers (true/false)
- `--uppercase BOOL` - Include uppercase letters (true/false)
- `--symbols BOOL` - Include symbols (true/false)

**Examples:**
```bash
# Generate with default settings
pass-cli password generate random

# Generate a 20-character password with all character types
pass-cli password generate random --length 20 --numbers true --uppercase true --symbols true

# Generate a simple password without symbols
pass-cli password generate random --length 16 --symbols false
```

#### generate memorable

Generate a memorable passphrase.

```bash
pass-cli password generate memorable [OPTIONS]
```

**Options:**
- `--words COUNT` - Number of words in the passphrase (default varies)
- `--separator CHAR` - Character to separate words (default varies)
- `--capitalize BOOL` - Capitalize words (true/false)
- `--numbers BOOL` - Include numbers (true/false)

**Examples:**
```bash
# Generate with default settings
pass-cli password generate memorable

# Generate a 5-word passphrase
pass-cli password generate memorable --words 5

# Generate with custom separator
pass-cli password generate memorable --words 4 --separator "-"

# Generate with numbers and capitalization
pass-cli password generate memorable --words 4 --capitalize true --numbers true
```

### score

Analyze password strength and security.

```bash
pass-cli password score PASSWORD [--output FORMAT]
```

**Arguments:**
- `PASSWORD` - The password to analyze (required)

**Options:**
- `--output FORMAT` - Output format: `human` (default) or `json`

**Examples:**
```bash
# Analyze a password
pass-cli password score "mypassword123"

# Get detailed analysis in JSON format
pass-cli password score "MySecureP@ssw0rd!" --output json

# Analyze a generated password
GENERATED=$(pass-cli password generate random --length 16)
pass-cli password score "$GENERATED"
```

## Password generation best practices

### Random passwords
- **Length**: Use at least 12 characters, preferably 16 or more
- **Character types**: Include uppercase, lowercase, numbers, and symbols
- **Uniqueness**: Generate unique passwords for each account
- **Storage**: Store in Proton Pass, don't try to memorize

### Memorable passphrases
- **Word count**: Use at least 4 words, preferably 5 or more
- **Randomness**: Let the generator choose words randomly
- **Separators**: Use separators to improve readability
- **Avoid patterns**: Don't use common phrases or patterns

## Password strength analysis

The `score` command analyzes several factors:

### Strength indicators
- **Length**: Longer passwords are generally stronger
- **Character diversity**: Multiple character types increase strength
- **Unpredictability**: Random patterns are stronger than predictable ones
- **Common patterns**: Avoids dictionary words and common substitutions

### Weakness detection
- **Dictionary words**: Common words reduce strength
- **Patterns**: Keyboard patterns (qwerty, 123456) are weak
- **Repetition**: Repeated characters or patterns
- **Personal information**: Birthdays, names, etc. (if detectable)

## Examples

### Password generation workflow

```bash
#!/bin/bash
echo "Generating various password types:"

echo "1. Strong random password:"
RANDOM_PASS=$(pass-cli password generate random --length 16 --uppercase true --symbols true)
echo "$RANDOM_PASS"
pass-cli password score "$RANDOM_PASS"

echo -e "\n2. Memorable passphrase:"
PASSPHRASE=$(pass-cli password generate memorable --words 5 --separator "-")
echo "$PASSPHRASE"
pass-cli password score "$PASSPHRASE"

echo -e "\n3. Simple random password:"
SIMPLE_PASS=$(pass-cli password generate random --length 12 --symbols false)
echo "$SIMPLE_PASS"
pass-cli password score "$SIMPLE_PASS"
```

### Password strength testing

```bash
#!/bin/bash
# Test various password patterns
passwords=(
    "password123"
    "P@ssw0rd!"
    "MyVeryLongAndSecurePassword2024"
    "correct-horse-battery-staple"
    "aB3$fG7*kL9#"
)

for pwd in "${passwords[@]}"; do
    echo "Testing: $pwd"
    pass-cli password score "$pwd"
    echo "---"
done
```

### Integration with item creation

```bash
#!/bin/bash
SHARE_ID="abc123def"

# Generate a strong password and create a login item
STRONG_PASSWORD=$(pass-cli password generate random --length 20 --uppercase true --symbols true)

# Verify the password strength
echo "Generated password strength:"
pass-cli password score "$STRONG_PASSWORD"

# Create login item with the generated password
pass-cli item create login \
  --share-id "$SHARE_ID" \
  --title "New Secure Account" \
  --username "myuser" \
  --password "$STRONG_PASSWORD" \
  --url "https://secure-site.com"
```

## Security considerations

### Password generation
- **Entropy**: Use sufficient randomness for security
- **Avoiding patterns**: Don't use predictable patterns
- **Length vs complexity**: Longer passwords are generally better than complex short ones
- **Context**: Consider the service's password requirements

### Password analysis
- **Sensitive data**: Be cautious when analyzing passwords in shared environments
- **Command history**: Passwords may be stored in shell history
- **Process visibility**: Other users might see passwords in process lists

### Best practices
- **Generate in secure environment**: Use trusted systems for password generation
- **Immediate storage**: Store generated passwords in Proton Pass immediately
- **Regular updates**: Generate new passwords periodically
- **Unique passwords**: Never reuse passwords across services

## Output formats

### Human-readable output
- Clear strength indicators
- Readable recommendations
- Color coding (if terminal supports it)
- Explanation of strength factors

### JSON output
- Structured data for automation
- Numeric strength scores
- Detailed analysis results
- Machine-parseable format

## Common use cases

### Account setup
Generate strong passwords when creating new accounts:
```bash
pass-cli password generate random --length 16 --uppercase true --symbols true
```

### Password policy compliance
Generate passwords that meet specific requirements:
```bash
# For services requiring symbols
pass-cli password generate random --length 12 --symbols true

# For services that don't allow symbols
pass-cli password generate random --length 16 --symbols false
```

### Security audits
Analyze existing passwords for weakness:
```bash
# Audit multiple passwords
for pwd in $(cat password_list.txt); do
    pass-cli password score "$pwd" --output json
done
```

## Related commands

- **[item create login](item.md#create-login)** - Create login items with generated passwords
- **[vault](vault.md)** - Manage vaults to store generated passwords
