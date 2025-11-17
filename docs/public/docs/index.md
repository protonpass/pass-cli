---
icon: lucide/rocket
---

# Overview

Welcome to the Proton Pass CLI documentation. The Proton Pass CLI is a command-line interface for managing your Proton Pass vaults, items, and secrets.

## Quick Start

- **[Installation](get-started/installation.md)** - Installation instructions for all platforms
- **[Getting Started](get-started/login.md)** - Login and configuration guides
- **[Pass Objects](objects/share.md)** - The different objects you can manage in Proton Pass
- **[Usage Guide](commands/login.md)** - Comprehensive guide to using the CLI

## What is Proton Pass CLI?

The Proton Pass CLI allows you to:

- **Manage vaults and items** - Create, list, view, and delete vaults and items from the command line
- **Inject secrets** - Use secrets in your applications via environment variables or template files
- **SSH integration** - Use Proton Pass-stored SSH keys with your existing SSH workflows
- **Automate workflows** - Integrate Proton Pass into your scripts and CI/CD pipelines

## Key Features

### Secure Authentication

- Support for password, TOTP, and FIDO2/WebAuthn authentication
- Flexible credential input via environment variables, files, or interactive prompts
- Secure session management

### Flexible Secret Management

- Reference secrets using a simple URI syntax: `pass://vault/item/field`
- Inject secrets into environment variables for your applications
- Process template files with secret references

### SSH Agent Integration

- Load SSH keys from Proton Pass into your existing SSH agent
- Run Proton Pass CLI as a standalone SSH agent
- Automatic key refresh and management

### Secure Key Storage

- Default keyring integration (macOS Keychain, Linux kernel keyring, Windows Credential Manager)
- Filesystem storage option for headless environments
- Encrypted session storage

## Need Help?

If you encounter any issues or have questions:

- Contact [support](https://proton.me/support/contact)
