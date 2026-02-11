# Agent Development Guide

This document outlines the development workflow and best practices for contributing to the Aliasman project.

## Quick Start

```bash
# Format code
cargo fmt

# Build the project
cargo build

# Run all tests
cargo test

# Run linter
cargo clippy

# Full check (run all of the above)
cargo fmt && cargo build && cargo test && cargo clippy
```

## Project Structure

Aliasman is organized as a Cargo workspace with two main crates:

```text
aliasman/
├── Cargo.toml                    # Workspace root
├── crates/
│   ├── aliasman-core/            # Core library
│   │   └── src/
│   │       ├── lib.rs            # Public API
│   │       ├── model.rs          # Data models
│   │       ├── error.rs          # Error types
│   │       ├── config.rs         # Configuration
│   │       ├── storage/          # Storage providers
│   │       │   ├── mod.rs
│   │       │   ├── sqlite.rs
│   │       │   └── s3.rs
│   │       └── email/            # Email providers
│   │           ├── mod.rs
│   │           └── rackspace.rs
│   └── aliasman-cli/             # CLI binary
│       └── src/
│           ├── main.rs
│           ├── commands/
│           │   ├── alias.rs
│           │   ├── config.rs
│           │   └── storage.rs
│           └── output.rs
```

## Development Workflow

### 1. Code Formatting

Always run `cargo fmt` before committing to ensure consistent code style:

```bash
# Format all code in the workspace
cargo fmt

# Check formatting without making changes
cargo fmt -- --check
```

### 2. Building

Build the entire workspace:

```bash
# Debug build (faster compilation, slower runtime)
cargo build

# Release build (slower compilation, optimized runtime)
cargo build --release

# Build specific crate
cargo build -p aliasman-core
cargo build -p aliasman-cli
```

### 3. Testing

Run the test suite:

```bash
# Run all tests
cargo test

# Run tests for specific crate
cargo test -p aliasman-core
cargo test -p aliasman-cli

# Run tests with output visible
cargo test -- --nocapture

# Run specific test
cargo test test_s3_alias_roundtrip

# Run ignored tests (e.g., integration tests requiring external services)
cargo test -- --ignored
```

### 4. Linting with Clippy

Run Clippy to catch common mistakes and improve code quality:

```bash
# Run clippy on all crates
cargo clippy

# Run clippy with all features enabled
cargo clippy --all-features

# Treat warnings as errors (useful in CI)
cargo clippy -- -D warnings

# Fix automatically applicable suggestions
cargo clippy --fix
```

## Common Development Tasks

### Running the CLI

```bash
# Run the CLI from the workspace
cargo run -- --help

# Run with specific command
cargo run -- alias list
cargo run -- alias create -d example.com -D "test" -r

# Run with custom config directory
cargo run -- --config-dir /path/to/config alias list
```

### Working with Storage Providers

#### SQLite (Default)

```bash
# Create default config (uses SQLite)
cargo run -- config

# The database will be at:
# - macOS: ~/Library/Application Support/aliasman/aliasman.db
# - Linux: ~/.config/aliasman/aliasman.db
```

#### S3 (Local Development with MinIO)

```bash
# Start MinIO locally
docker run -p 9000:9000 -p 9001:9001 \
  -e MINIO_ROOT_USER=minioadmin \
  -e MINIO_ROOT_PASSWORD=minioadmin \
  minio/minio server /data --console-address ":9001"

# Configure aliasman to use local MinIO
# See config examples in README.md
```

### Storage Conversion

```bash
# Convert between storage systems
cargo run -- storage convert --source home --destination s3-production

# Convert from legacy Go S3 format
cargo run -- storage convert --source legacy --destination new --legacy-source
```

## Best Practices

### Before Submitting Changes

Always run the full verification suite:

```bash
#!/bin/bash
# verify.sh - Run this before submitting PRs

set -e

echo "=== Formatting code ==="
cargo fmt

echo "=== Building project ==="
cargo build --all-features

echo "=== Running tests ==="
cargo test --all-features

echo "=== Running clippy ==="
cargo clippy --all-features -- -D warnings

echo "=== All checks passed! ==="
```

### Code Quality Guidelines

1. **Error Handling**: Use `thiserror` for library errors, `anyhow` for CLI
2. **Async**: All I/O operations should be async using tokio
3. **Documentation**: Document public APIs with rustdoc comments
4. **Testing**: Write tests for new functionality
5. **Clippy**: Address all clippy warnings before submitting

### Storage Provider Implementation

When adding a new storage provider:

1. Implement the `StorageProvider` trait in `crates/aliasman-core/src/storage/`
2. Add variant to `StorageConfig` in `config.rs`
3. Update factory function in `lib.rs`
4. Add tests following existing patterns
5. Update README.md with configuration examples

### Email Provider Implementation

When adding a new email provider:

1. Implement the `EmailProvider` trait in `crates/aliasman-core/src/email/`
2. Add variant to `EmailConfig` in `config.rs`
3. Update factory function in `lib.rs`
4. Add tests with mock implementations

## CI/CD Expectations

The CI pipeline typically runs:

```bash
cargo fmt -- --check
cargo build --all-features
cargo test --all-features
cargo clippy --all-features -- -D warnings
```

Ensure all commands pass locally before pushing.

## Troubleshooting

### Build Issues

```bash
# Clean build artifacts
cargo clean

# Update dependencies
cargo update

# Check for outdated dependencies
cargo outdated  # requires cargo-outdated
```

### Test Issues

```bash
# Run tests single-threaded (for debugging)
cargo test -- --test-threads=1

# Run with backtrace on failure
RUST_BACKTRACE=1 cargo test
```

### Database Issues (SQLite)

```bash
# SQLite database is locked - check for zombie processes
lsof ~/.config/aliasman/*.db

# Reset database (WARNING: deletes all data)
rm ~/.config/aliasman/*.db
```

## Additional Tools

### Useful Cargo Commands

```bash
# Check code without building
cargo check

# Generate documentation
cargo doc --open

# Show dependency tree
cargo tree

# Run benchmarks (if any)
cargo bench

# Check for security vulnerabilities
cargo audit  # requires cargo-audit
```

### IDE Support

For VS Code, recommended extensions:

- rust-analyzer
- Even Better TOML
- CodeLLDB (for debugging)

## Resources

- [Rust Book](https://doc.rust-lang.org/book/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Cargo Book](https://doc.rust-lang.org/cargo/)
- [Rust Async Book](https://rust-lang.github.io/async-book/)
