# Aliasman

Email alias management tool with pluggable storage and email providers. Rust rewrite of a Go original.

## Build & Test

```bash
cargo build                                                     # Build entire workspace
cargo test                                                      # Run all tests
cargo test -p aliasman-core                                     # Test a single crate
cargo clippy --all-features --all-targets                       # Check code quality
cargo clippy --all-features --all-targets --allow-dirty --fix   # Auto-fix clippy warnings before fixing manually
cargo fmt                                                       # Format code
cargo run --package aliasman-cli -- alias list                  # Run the CLI
cargo run --package aliasman-web                                # Run the web frontend
```

- All tests must pass before committing
- `cargo clippy` must pass with no warnings
- `cargo fmt` must pass with no formatting changes

## Commit Style

- Prefix: `feat:`, `fix:`, `refactor:`, `test:`, `docs:`
- Summary line under 80 chars with counts where relevant
- Body: each logical change on its own `-` bullet
- Summarize what was added/changed and why, not just file names

## Architecture

- **Workspace** with three crates: `aliasman-core` (library), `aliasman-cli` (CLI binary), `aliasman-web` (web binary)
- Both binaries depend on `aliasman-core` for all domain logic
- Core exposes two traits: `StorageProvider` and `EmailProvider`, implemented via enum dispatch
- Mutations always dual-write: update the email provider first, then persist to storage
- Fully async (tokio) — all I/O uses `async_trait`
- Config: TOML at `~/.config/aliasman/config.toml`, loaded via the `config` crate with env var override (`ALIASMAN_` prefix)
- Multi-system: each named system pairs one storage provider + one email provider

## Conventions

- **Library errors**: `thiserror` in `aliasman-core` — never use `anyhow` in the core crate
- **Binary errors**: `anyhow` in CLI and web crates — convert core errors with `?`
- **Provider config**: `#[serde(tag = "type")]` enum variants — not trait objects, not dynamic registration
- **Adding a provider**: implement the trait, add an enum variant to config, add a match arm in the factory function in `lib.rs`
- **Default config dir**: `dirs::config_dir()` (macOS: `~/Library/Application Support`), joined with `aliasman`
- **Path expansion**: use `AppConfig::expand_path()` (shellexpand) for user-facing paths with `~`

## Gotchas

- SQLite tests use `":memory:"` as the path — never use real file paths in tests
- `create_storage_provider_legacy()` exists only for Go-format S3 migration — don't use it for new code
- `alias_matches()` intentionally ignores `modified_at` — this is by design for storage conversion diffing
- Web `AppState` wraps storage providers in `RwLock<HashMap>` — always use `AppState` methods, never lock manually
- `StorageProvider::open()` must be called before any queries — `open(true)` for read-only, `open(false)` for read-write
- `StorageProvider::close()` must be called to flush writes (triggers S3 index upload)
- The dual-write order matters: email provider first, then storage — if email fails, storage stays consistent
