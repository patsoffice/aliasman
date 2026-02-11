use anyhow::{Context, Result};
use clap::Subcommand;

use aliasman_core::config::AppConfig;
use aliasman_core::{convert_storage, create_storage_provider, create_storage_provider_legacy};

#[derive(Subcommand)]
pub enum StorageCommands {
    /// Convert aliases between storage systems
    Convert {
        /// Source system name
        #[arg(long, short)]
        source: String,

        /// Destination system name
        #[arg(long, short)]
        destination: String,

        /// Use legacy format reader for source (Go S3 format)
        #[arg(long)]
        legacy_source: bool,
    },
}

pub async fn handle(command: &StorageCommands, config_dir: &std::path::Path) -> Result<()> {
    match command {
        StorageCommands::Convert {
            source,
            destination,
            legacy_source,
        } => {
            handle_convert(config_dir, source, destination, *legacy_source).await?;
        }
    }
    Ok(())
}

async fn handle_convert(
    config_dir: &std::path::Path,
    source_name: &str,
    dest_name: &str,
    legacy_source: bool,
) -> Result<()> {
    let config = AppConfig::load(config_dir).context("failed to load configuration")?;

    // Get source and destination systems
    let source_system = config
        .system(Some(source_name))
        .context(format!("source system '{}' not found", source_name))?;

    let dest_system = config
        .system(Some(dest_name))
        .context(format!("destination system '{}' not found", dest_name))?;

    // Create storage providers
    let mut source_storage = if legacy_source {
        create_storage_provider_legacy(&source_system.storage)
    } else {
        create_storage_provider(&source_system.storage)
    };

    let mut dest_storage = create_storage_provider(&dest_system.storage);

    // Perform conversion
    let result = convert_storage(source_storage.as_mut(), dest_storage.as_mut())
        .await
        .context("storage conversion failed")?;

    // Print results
    println!("Storage conversion complete:");
    println!("  Total aliases: {}", result.total);
    println!("  Inserted: {}", result.inserted);
    println!("  Updated: {}", result.updated);
    println!("  Skipped: {}", result.skipped);

    Ok(())
}
