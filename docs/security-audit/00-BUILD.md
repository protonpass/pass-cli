# Building Proton Pass CLI

This document explains how to build the Proton Pass CLI from source with vendored dependencies.

## Prerequisites

The following tools are required to build the project:

- **Rust toolchain** (1.89 or later recommended)
- **Cargo** (comes with Rust)

### Installing prerequisites

**On macOS:**
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

```

**On Linux (Ubuntu/Debian):**
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install build dependencies
sudo apt-get update
sudo apt-get install -y pkg-config libssl-dev libdbus-1-dev
```

**On Linux (RHEL/Fedora):**
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install build dependencies
sudo dnf install -y pkg-config openssl-devel dbus-devel systemd-devel
```

## Building with vendored dependencies

Since you have been provided with a fresh copy of the repository with vendored dependencies, the build process is straightforward.

To build the release version of the CLI:

```bash
cargo build --release
```

You may find the compiled binary will be located at:
```
target/release/pass-cli
```

And for testing that the binary works:

```bash
./target/release/pass-cli --version
```

## Project structure

The project is organized as a Cargo workspace with the following crates:

- **pass-cli** - The main CLI application
- **pass** - Core library with business logic and API client
- **pass-domain** - Domain models and traits
- **pass-fs** - Filesystem abstraction layer
- **pass-pgp** - PGP cryptography implementation

## Build features

The project uses several Cargo features that can be enabled during compilation:

- `keyring` (enabled by default) - Enables system keyring integration for securely storing the local encryption key
- `internal` - Enables internal testing commands (not recommended for production)
- `no-login-restriction` - Disables login restrictions for testing. Currently only paid accounts are allowed to login. If this is disabled the client will not make the check.

To build without default features:

```bash
cargo build --release --no-default-features
```

And to build with some features enabled:

```bash
cargo build --release --features no-login-restriction --features keyring
```

We recommend at least enabling `no-login-restriction` to make sure you can log in with any account you create from the web.

## Development builds

For development purposes and faster builds, you can build without optimizations:

```bash
cargo build
```

The debug binary will be at `target/debug/pass-cli`. Same flags can be used for feature enabling.

## Troubleshooting

### Missing system libraries on Linux

If you encounter errors about missing system libraries, install:
- `libdbus-1-3` or `dbus-libs`

