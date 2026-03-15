mod commands;
mod output;

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use aliasman_core::config::AppConfig;
use aliasman_core::create_providers;

#[derive(Parser)]
#[command(name = "aliasman", about = "Email alias manager")]
struct Cli {
    /// Configuration directory
    #[arg(long, default_value_os_t = AppConfig::default_config_dir())]
    config_dir: PathBuf,

    /// System to use (overrides default_system in config)
    #[arg(long, short)]
    system: Option<String>,

    /// Show what would be done without making any changes
    #[arg(long)]
    dry_run: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Manage email aliases
    Alias {
        #[command(subcommand)]
        command: commands::alias::AliasCommands,
    },

    /// Generate a default configuration file
    Config,

    /// Print version information
    Version,

    /// Storage management commands
    Storage {
        #[command(subcommand)]
        command: commands::storage::StorageCommands,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Config => {
            commands::config::handle(&cli.config_dir)?;
        }

        Commands::Version => {
            println!("aliasman {}", env!("CARGO_PKG_VERSION"));
        }

        Commands::Alias { command } => {
            let config =
                AppConfig::load(&cli.config_dir).context("failed to load configuration")?;
            let system = config
                .system(cli.system.as_deref())
                .context("failed to resolve system")?;

            let (mut storage, email) =
                create_providers(system).context("failed to create providers")?;

            commands::alias::handle(
                command,
                storage.as_mut(),
                email.as_ref(),
                system.domain.as_deref(),
                system.email_addresses.as_deref(),
                cli.dry_run,
            )
            .await?;
        }

        Commands::Storage { command } => {
            commands::storage::handle(command, &cli.config_dir).await?;
        }
    }

    Ok(())
}
