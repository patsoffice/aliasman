# Aliasman

Aliasman is a tool for managing a large number of email aliases, with both a CLI and a web frontend. It supports pluggable storage and email providers, allowing you to manage alias metadata locally while controlling the actual email routing through your email service provider.

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
│   │       │   ├── sqlite.rs     # SQLite implementation (sqlx)
│   │       │   └── s3.rs         # S3 implementation (aws-sdk-s3)
│   │       └── email/
│   │           ├── mod.rs        # EmailProvider trait
│   │           └── rackspace.rs  # Rackspace Email implementation
│   ├── aliasman-cli/             # Binary: CLI frontend
│   │   └── src/
│   │       ├── main.rs
│   │       ├── commands/
│   │       │   ├── mod.rs
│   │       │   ├── alias.rs      # alias create/edit/delete/list/suspend/search
│   │       │   ├── config.rs     # config command
│   │       │   └── storage.rs    # storage convert command
│   │       └── output.rs         # Table formatting (comfy-table)
│   └── aliasman-web/             # Binary: web frontend
│       ├── src/
│       │   ├── main.rs           # Axum server, CLI args, startup
│       │   ├── routes.rs         # HTTP handlers and Askama templates
│       │   ├── state.rs          # Shared application state
│       │   ├── error.rs          # Web error type
│       │   └── auth.rs           # Auth/RBAC stubs (future)
│       ├── templates/            # Askama HTML templates
│       └── static/               # Embedded static assets (htmx.min.js)
```

### Design Decisions

- **Fully async** — tokio runtime throughout, including storage. The rackspace-email crate is async and the future web frontend benefits from this.
- **Workspace with lib + bins** — Core logic lives in `aliasman-core` so both the CLI and web frontend consume it.
- **Enum dispatch for providers** — Provider selection uses Rust enums with `serde(tag = "type")` rather than dynamic registration. Type-safe and exhaustive at compile time.
- **Dual-write pattern** — Mutations (create, edit, delete, suspend, unsuspend) write to both the email provider and storage provider. The email provider manages actual email routing; storage maintains metadata, timestamps, and descriptions.
- **Testable provider wrappers** — External API clients (e.g. `RackspaceClient`) are wrapped behind internal traits (`RackspaceClientImpl`) so they can be replaced with mocks in tests without hitting real services.
- **thiserror + anyhow** — `thiserror` for typed errors in the library, `anyhow` for ergonomic error propagation in the CLI binary.
- **TOML configuration** — Config file at `~/.config/aliasman/config.toml`, loaded via the `config` crate with environment variable overrides.
- **Multi-system support** — Configuration supports multiple named "systems" (e.g. "home", "work"), each pairing a storage and email provider with per-system defaults. Select via `--system` flag or `default_system` in config.

### Providers

**Storage providers** manage alias metadata (descriptions, timestamps, suspension state):

| Provider | Status      | Description                                               |
|----------|-------------|-----------------------------------------------------------|
| `sqlite` | Implemented | SQLite via sqlx. Default: `~/.config/aliasman/aliasman.db` |
| `s3`     | Implemented | AWS S3 with per-alias objects and an index blob           |
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
| `aws-sdk-s3` | AWS S3 SDK |
| `aws-config` | AWS configuration and credentials |
| `serde_json` | JSON serialization for S3 storage |
| `rand` | Random alias generation |
| `axum` | Web server framework |
| `askama` | Compile-time HTML templates |
| `rust-embed` | Embed static assets in binary |
| `htmx` | Dynamic interactions (JS, embedded) |

## Installing

### From Source

```sh
# CLI
cargo install --path crates/aliasman-cli

# Web frontend
cargo install --path crates/aliasman-web
```

## Configuring

Create a configuration file at `~/.config/aliasman/config.toml`, or run `aliasman config`
to generate a starter file.

`default_system` selects which system is used when `--system` is not specified:

```toml
default_system = "home"
```

Each system pairs a storage provider with an email provider, and can set a default domain
and email addresses. SQLite is the simplest storage option, using a local database file:

```toml
[systems.home]
domain = "example.com"
email_addresses = ["person@example.com"]

[systems.home.storage]
type = "sqlite"
db_path = "~/.config/aliasman/home.db"

[systems.home.email]
type = "rackspace"
user_key = "your-api-user-key"
secret_key = "your-api-secret-key"
```

You can define multiple named systems (e.g. "home" and "work") and switch between them
with `--system`:

```toml
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

S3 storage uses the standard AWS credential chain (environment variables,
`~/.aws/credentials`, IAM roles, etc.) so no credentials need to be stored in the config
file:

```toml
[systems.s3-example.storage]
type = "s3"
bucket = "my-aliasman-bucket"
region = "us-east-1"
```

Static credentials can be provided for S3-compatible services like MinIO or LocalStack,
including a custom endpoint:

```toml
[systems.s3-local.storage]
type = "s3"
bucket = "aliasman-bucket"
region = "us-east-1"
endpoint = "http://localhost:9000"
access_key_id = "minioadmin"
secret_access_key = "minioadmin"
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

Edit an alias's email addresses or description:

```sh
# Change the description
aliasman alias edit -a 5f888d1272833b09 -D "new description"

# Change the target email addresses
aliasman alias edit -a 5f888d1272833b09 -e newperson@example.com

# Change both
aliasman alias edit -a 5f888d1272833b09 -e newperson@example.com -D "new description"
```

Delete an alias:

```sh
aliasman alias delete -a 5f888d1272833b09 -d example.com
```

Suspend an alias (stops email routing but preserves metadata):

```sh
aliasman alias suspend -a 5f888d1272833b09 -d example.com
```

Unsuspend an alias (restarts email routing):

```sh
aliasman alias unsuspend -a 5f888d1272833b09 -d example.com
```

Preview any mutation without making changes:

```sh
aliasman --dry-run alias create -r -D "test"
aliasman --dry-run alias edit -a 5f888d1272833b09 -D "new description"
aliasman --dry-run alias delete -a 5f888d1272833b09
```

Search aliases with a regular expression (matches against alias, domain, email addresses,
and description):

```sh
aliasman alias search -s "shopping"

# Show only suspended aliases
aliasman alias search --exclude-enabled

# Show only active aliases
aliasman alias search --exclude-suspended

# Combine a search pattern with a filter
aliasman alias search -s "example\\.com" --exclude-suspended
```

Audit aliases by comparing storage against the email provider:

```sh
# Audit the default domain
aliasman audit

# Audit a specific domain
aliasman audit -d example.com
```

Reports three types of differences:

- **MISSING FROM EMAIL** — alias is active in storage but not on the email provider
- **MISSING FROM STORAGE** — alias exists on the email provider but is not tracked in storage
- **ADDRESS MISMATCH** — alias exists in both but the target email addresses differ

Suspended aliases are expected to be absent from the email provider and are not flagged.

Use a specific system:

```sh
aliasman --system work alias list
```

### Web Frontend

Start the web server:

```sh
aliasman-web
```

This starts the web UI at `http://127.0.0.1:3000` using your existing
`~/.config/aliasman/config.toml`. The UI provides:

- Alias table displaying `alias@domain` with search and filtering (powered by HTMX)
- Create aliases with an inline form and random name generator
- Edit alias email addresses and descriptions inline
- Suspend, unsuspend, and delete aliases with per-row action buttons
- System switcher dropdown for multi-system configs
- Hide suspended / hide enabled toggles
- Manual refresh button and automatic 60-second polling

Options:

```sh
# Custom config directory
aliasman-web --config-dir /path/to/config

# Custom bind address
aliasman-web --bind 0.0.0.0:8080
```

### Storage Conversion

Convert aliases between storage systems:

```sh
# Convert from SQLite to S3
aliasman storage convert --source home --destination s3-example

# Convert from legacy Go S3 format to new S3 format
aliasman storage convert --source legacy-s3 --destination s3-new --legacy-source

# Convert from S3 to SQLite
aliasman storage convert --source s3-example --destination home
```

The `--legacy-source` flag enables reading from the legacy Go S3 format (metadata stored in S3 object headers). This is useful for migrating from the original Go implementation of aliasman.

Full help is available for all commands and subcommands with `--help`.

## Storage Formats

### SQLite Storage

SQLite is the default storage provider, storing aliases in a local database file.

### S3 Storage (New Rust Format)

The new Rust-native S3 format stores aliases as JSON objects with the following structure:

- **Object key**: `alias-{alias}@{domain}` (e.g., `alias-shopping@example.com`)
- **JSON body**: Contains all alias fields with Rust naming conventions
  - `alias`: The alias name
  - `domain`: The domain
  - `email_addresses`: Array of email addresses
  - `description`: Description text
  - `suspended`: Boolean flag
  - `created_at`: RFC3339 timestamp
  - `modified_at`: RFC3339 timestamp
  - `suspended_at`: RFC3339 timestamp or `null`
- **Index object**: An `index` object stores a JSON array of all aliases for fast loading

### Legacy Go S3 Format

The original Go implementation stored aliases differently:

- **Object key**: Same format (`alias-{alias}@{domain}`)
- **Object body**: Empty (0 bytes)
- **Metadata headers**: All data stored in S3 object metadata
  - `alias`, `domain`, `description`, `email_addresses` (comma-separated)
  - `suspended` ("true"/"false")
  - `created_ts`, `modified_ts`, `suspended_ts` (RFC3339)
- **Go zero time**: `"0001-01-01T00:00:00Z"` represents unset timestamps

Use `--legacy-source` flag when converting from the old format.

## Planned Features

- **Additional CLI commands** — sync, sync-from-email
- **Additional providers** — files storage, Google Workspace email
- **Web authentication** — Login flow with RBAC (role-based access control) for multi-user deployments
