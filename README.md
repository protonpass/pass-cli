# Proton Pass CLI

A command-line interface for [Proton Pass](https://proton.me/pass).

## Documentation

Full documentation is available at **https://protonpass.github.io/pass-cli/**

## Building from source

**Prerequisites:** [Rust toolchain](https://rustup.rs/) (stable)

```bash
# Debug build
cargo build

# Release (optimized) build
cargo build --release
```

The compiled binary will be at `target/debug/pass-cli` or `target/release/pass-cli` respectively.

## Development

### Running tests

```bash
cargo test
```

To run tests for a specific crate:

```bash
cargo test -p pass-cli
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).
