# aliasman-cli

CLI binary using clap derive macros and `anyhow` for error handling.

## Rules

- Error handling: `anyhow` with `.context()` — convert core errors via `?`
- All command handlers live in `commands/` — one file per top-level subcommand
- Table output goes through `output.rs` using `comfy-table`
- Top-level args (`--config-dir`, `--system`) are on the `Cli` struct; subcommand args are on their own structs

## Adding a Command

1. Create `commands/<name>.rs` with a clap `#[derive(Subcommand)]` enum and a `handle()` async fn
2. Add a variant to the `Commands` enum in `main.rs`
3. Add dispatch in the `main()` match block
4. If the command needs providers, follow the `Commands::Alias` pattern: load config, resolve system, create providers

## CLI Flow

1. Parse args (`Cli::parse()`)
2. Load config from `--config-dir`
3. Resolve system (`--system` flag or `default_system` in config)
4. Create providers via `create_providers()`
5. Dispatch to command handler
