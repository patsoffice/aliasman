# Aliasman

Aliasman is a CLI tool (with a planned web frontend) for managing a large number of email aliases. It supports pluggable storage and email providers, allowing you to manage alias metadata locally while controlling the actual email routing through your email service provider.

## Architecture

Aliasman is structured as a Cargo workspace with separate crates for the core library and frontends:

```text
aliasman/
├── Cargo.toml                    # Workspace root
├── crates/
│   ├── aliasman-core/            # Library: models, traits, providers, business logic
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── model.rs          # Alias, AliasFilter
│   │       ├── error.rs          # Typed error types (thiserror)
│   │       ├── config.rs         # AppConfig with serde + config crate
│   │       ├── storage/
│   │       │   ├── mod.rs        # StorageProvider trait
│   │       │   └── sqlite.rs     # SQLite implementation (sqlx)
│   │       └── email/
│   │           ├── mod.rs        # EmailProvider trait
│   │           └── rackspace.rs  # Rackspace Email implementation
│   └── aliasman-cli/             # Binary: CLI frontend
│       └── src/
│           ├── main.rs
│           ├── commands/
│           │   ├── mod.rs
│           │   ├── alias.rs      # alias create/delete/list
│           │   └── config.rs     # config command
│           └── output.rs         # Table formatting (comfy-table)
```

### Design Decisions

- **Fully async** — tokio runtime throughout, including storage. The rackspace-email crate is async and the future web frontend benefits from this.
- **Workspace with lib + bin** — Core logic lives in `aliasman-core` so both the CLI and a future web frontend can consume it.
- **Enum dispatch for providers** — Provider selection uses Rust enums with `serde(tag = "type")` rather than dynamic registration. Type-safe and exhaustive at compile time.
- **Dual-write pattern** — Mutations (create, delete, suspend, unsuspend) write to both the email provider and storage provider. The email provider manages actual email routing; storage maintains metadata, timestamps, and descriptions.
- **Testable provider wrappers** — External API clients (e.g. `RackspaceClient`) are wrapped behind internal traits (`RackspaceClientImpl`) so they can be replaced with mocks in tests without hitting real services.
- **thiserror + anyhow** — `thiserror` for typed errors in the library, `anyhow` for ergonomic error propagation in the CLI binary.
- **TOML configuration** — Config file at `~/.config/aliasman/config.toml`, loaded via the `config` crate with environment variable overrides.
- **Multi-system support** — Configuration supports multiple named "systems" (e.g. "home", "work"), each pairing a storage and email provider with per-system defaults. Select via `--system` flag or `default_system` in config.

### Providers

**Storage providers** manage alias metadata (descriptions, timestamps, suspension state):

| Provider | Status      | Description                                               |
|----------|-------------|-----------------------------------------------------------|
| `sqlite` | Implemented | SQLite via sqlx. Default: `~/.config/aliasman/aliasman.db` |
| `s3`     | Planned     | AWS S3 with per-alias objects and an index blob           |
| `files`  | Planned     | JSON files on the local filesystem                        |

**Email providers** manage actual email routing:

| Provider    | Status      | Description                                      |
|-------------|-------------|--------------------------------------------------|
| `rackspace` | Implemented | Rackspace Email API via the `rackspace-email` crate |
| `gsuite`    | Planned     | Google Workspace Admin API                       |

### Core Traits

```rust
#[async_trait]
pub trait StorageProvider: Send + Sync {
    async fn open(&mut self, read_only: bool) -> Result<()>;
    async fn close(&mut self) -> Result<()>;
    async fn get(&self, alias: &str, domain: &str) -> Result<Option<Alias>>;
    async fn put(&self, alias: &Alias) -> Result<()>;
    async fn update(&self, alias: &Alias) -> Result<()>;
    async fn delete(&self, alias: &str, domain: &str) -> Result<()>;
    async fn search(&self, filter: &AliasFilter) -> Result<Vec<Alias>>;
    async fn suspend(&self, alias: &str, domain: &str) -> Result<()>;
    async fn unsuspend(&self, alias: &str, domain: &str) -> Result<()>;
}

#[async_trait]
pub trait EmailProvider: Send + Sync {
    async fn alias_create(&self, alias: &str, domain: &str, addresses: &[String]) -> Result<()>;
    async fn alias_delete(&self, alias: &str, domain: &str) -> Result<()>;
    async fn alias_list(&self, domain: &str) -> Result<Vec<Alias>>;
}
```

### Key Dependencies

| Crate | Purpose |
| --- | --- |
| `tokio` | Async runtime |
| `clap` (derive) | CLI argument parsing |
| `serde` + `toml` | Config serialization |
| `config` | Layered config loading (files, env vars) |
| `sqlx` (sqlite) | Async SQLite storage |
| `rackspace-email` | Rackspace Email API client |
| `chrono` | Timestamps |
| `regex` | Alias filtering |
| `comfy-table` | CLI table output |
| `thiserror` | Library error types |
| `anyhow` | CLI error handling |
| `async-trait` | Async trait support |
| `rand` | Random alias generation |

## Installing

### From Source

```sh
cargo install --path crates/aliasman-cli
```

## Configuring

Create a configuration file at `~/.config/aliasman/config.toml`, or run `aliasman config`
to generate a starter file.

The config supports multiple named systems. Each system pairs a storage provider with an
email provider and can have its own default domain and email addresses:

```toml
default_system = "home"

[systems.home]
domain = "example.com"
email_addresses = ["person@example.com"]

[systems.home.storage]
type = "sqlite"
db_path = "~/.config/aliasman/home.db"

[systems.home.email]
type = "rackspace"
user_key = "your-home-api-user-key"
secret_key = "your-home-api-secret-key"

[systems.work]
domain = "work.com"
email_addresses = ["me@work.com"]

[systems.work.storage]
type = "sqlite"
db_path = "~/.config/aliasman/work.db"

[systems.work.email]
type = "rackspace"
user_key = "your-work-api-user-key"
secret_key = "your-work-api-secret-key"
```

Use `--system work` to target a specific system, or omit it to use `default_system`.

## Using

Create an alias with a random name:

```sh
aliasman alias create -d example.com -D "company.com" -r -e person1@example.com -e person2@example.com
```

Output:

```text
Created alias 5f888d1272833b09@example.com -> person1@example.com, person2@example.com
```

List all aliases:

```sh
aliasman alias list
```

Delete an alias:

```sh
aliasman alias delete -a 5f888d1272833b09 -d example.com
```

Use a specific system:

```sh
aliasman --system work alias list
```

Full help is available for all commands and subcommands with `--help`.

## Planned Features

- **Additional CLI commands** — suspend, unsuspend, search, audit, sync, sync-from-email, update-description
- **Additional providers** — S3 storage, files storage, Google Workspace email
- **Web frontend** — A separate binary crate serving a web UI, consuming the same `aliasman-core` library
