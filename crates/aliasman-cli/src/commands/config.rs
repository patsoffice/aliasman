use std::path::Path;

use anyhow::{Context, Result};

use aliasman_core::write_default_config;

pub fn handle(config_dir: &Path) -> Result<()> {
    let config_file = config_dir.join("config.toml");
    if config_file.exists() {
        println!(
            "Configuration file already exists at {}",
            config_file.display()
        );
        println!("Edit it directly to make changes.");
        return Ok(());
    }

    write_default_config(config_dir).context("failed to write default config")?;
    println!("Default configuration written to {}", config_file.display());
    println!("Edit it with your provider credentials and settings.");

    Ok(())
}
