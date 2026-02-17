# aliasman-core

Core library crate — all domain logic, data models, traits, and provider implementations.

## Rules

- All errors use `thiserror` via `error::Error` enum — never `anyhow`, never `unwrap()` in non-test code
- All I/O operations must be async (`#[async_trait]`)
- Public API surface is defined in `lib.rs` — re-export anything downstream crates need
- Traits: `StorageProvider` in `storage/mod.rs`, `EmailProvider` in `email/mod.rs`
- Implementations go in sibling files: `storage/sqlite.rs`, `storage/s3.rs`, `email/rackspace.rs`
- Error conversions from external crate errors (sqlx, rackspace_email, serde_json) use `From` impls in `error.rs`

## Adding a Storage Provider

1. Create `storage/<name>.rs` implementing `StorageProvider`
2. Add a variant to `StorageConfig` in `config.rs` with `#[serde(rename = "<name>")]`
3. Add a match arm in `create_storage_provider()` in `lib.rs`
4. Write tests — use in-memory backends or tempfiles, never real paths

## Adding an Email Provider

1. Create `email/<name>.rs` implementing `EmailProvider`
2. Add a variant to `EmailConfig` in `config.rs` with `#[serde(rename = "<name>")]`
3. Add a match arm in `create_email_provider()` in `lib.rs`

## Tests

- `cargo test -p aliasman-core`
- SQLite tests use `":memory:"` — each test gets a fresh DB via `SqliteStorage::new(":memory:")`
- Config tests use `toml::from_str()` to parse inline TOML and `tempfile::tempdir()` for file-based tests
