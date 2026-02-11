pub mod config;
pub mod email;
pub mod error;
pub mod model;
pub mod storage;

use chrono::Utc;
use std::path::Path;

use crate::config::{AppConfig, EmailConfig, StorageConfig, SystemConfig};
use crate::email::rackspace::RackspaceEmailProvider;
use crate::email::EmailProvider;
use crate::error::{Error, Result};
use crate::model::{Alias, AliasFilter};
use crate::storage::s3::S3Storage;
use crate::storage::sqlite::SqliteStorage;
use crate::storage::StorageProvider;

/// Create the appropriate storage provider for a system.
pub fn create_storage_provider(config: &StorageConfig) -> Box<dyn StorageProvider> {
    match config {
        StorageConfig::Sqlite { db_path } => {
            let expanded = AppConfig::expand_path(db_path);
            Box::new(SqliteStorage::new(&expanded))
        }
        StorageConfig::S3 {
            bucket,
            region,
            endpoint,
            access_key_id,
            secret_access_key,
        } => Box::new(S3Storage::new(
            bucket,
            region.clone(),
            endpoint.clone(),
            access_key_id.clone(),
            secret_access_key.clone(),
            false, // legacy_mode = false
        )),
    }
}

/// Create a storage provider in legacy mode (for reading old Go S3 format during conversion).
pub fn create_storage_provider_legacy(config: &StorageConfig) -> Box<dyn StorageProvider> {
    match config {
        StorageConfig::S3 {
            bucket,
            region,
            endpoint,
            access_key_id,
            secret_access_key,
        } => Box::new(S3Storage::new(
            bucket,
            region.clone(),
            endpoint.clone(),
            access_key_id.clone(),
            secret_access_key.clone(),
            true, // legacy_mode = true
        )),
        // For non-S3 providers, just use the regular factory
        _ => create_storage_provider(config),
    }
}

/// Create the appropriate email provider for a system.
pub fn create_email_provider(config: &EmailConfig) -> Result<Box<dyn EmailProvider>> {
    match config {
        EmailConfig::Rackspace {
            user_key,
            secret_key,
        } => Ok(Box::new(RackspaceEmailProvider::new(user_key, secret_key)?)),
    }
}

/// Create both providers from a system config.
pub fn create_providers(
    system: &SystemConfig,
) -> Result<(Box<dyn StorageProvider>, Box<dyn EmailProvider>)> {
    let storage = create_storage_provider(&system.storage);
    let email = create_email_provider(&system.email)?;
    Ok((storage, email))
}

/// Create a new alias on both the email provider and storage.
pub async fn create_alias(
    storage: &dyn StorageProvider,
    email: &dyn EmailProvider,
    alias: Alias,
) -> Result<Alias> {
    if storage.get(&alias.alias, &alias.domain).await?.is_some() {
        return Err(Error::AliasAlreadyExists {
            alias: alias.alias.clone(),
            domain: alias.domain.clone(),
        });
    }

    // Create on the email provider first
    email
        .alias_create(&alias.alias, &alias.domain, &alias.email_addresses)
        .await?;

    // Then persist to storage
    storage.put(&alias).await?;

    Ok(alias)
}

/// Delete an alias from both the email provider and storage.
pub async fn delete_alias(
    storage: &dyn StorageProvider,
    email: &dyn EmailProvider,
    alias: &str,
    domain: &str,
) -> Result<()> {
    let existing = storage.get(alias, domain).await?;
    if existing.is_none() {
        return Err(Error::AliasNotFound {
            alias: alias.to_string(),
            domain: domain.to_string(),
        });
    }

    // Delete from email provider first
    email.alias_delete(alias, domain).await?;

    // Then remove from storage
    storage.delete(alias, domain).await?;

    Ok(())
}

/// List all aliases from storage, optionally filtered.
pub async fn list_aliases(
    storage: &dyn StorageProvider,
    filter: &AliasFilter,
) -> Result<Vec<Alias>> {
    storage.search(filter).await
}

/// Build a new Alias struct with sensible defaults for timestamps.
pub fn build_alias(
    alias_name: String,
    domain: String,
    email_addresses: Vec<String>,
    description: String,
) -> Alias {
    let now = Utc::now();
    Alias {
        alias: alias_name,
        domain,
        email_addresses,
        description,
        suspended: false,
        created_at: now,
        modified_at: now,
        suspended_at: None,
    }
}

/// Result of a storage conversion operation.
#[derive(Debug)]
pub struct ConvertResult {
    pub total: usize,
    pub inserted: usize,
    pub updated: usize,
    pub skipped: usize,
}

/// Convert aliases from one storage provider to another.
///
/// 1. Opens the source storage read-only and reads all aliases
/// 2. Opens the destination storage read-write
/// 3. For each alias: skips if identical exists, updates if differs, inserts if missing
/// 4. Closes both storages (destination write triggers index update for S3)
pub async fn convert_storage(
    source: &mut dyn StorageProvider,
    dest: &mut dyn StorageProvider,
) -> Result<ConvertResult> {
    // Open source read-only and read all aliases
    source.open(true).await?;
    let source_aliases = source.search(&AliasFilter::default()).await?;
    source.close().await?;

    // Open destination read-write
    dest.open(false).await?;

    let mut result = ConvertResult {
        total: source_aliases.len(),
        inserted: 0,
        updated: 0,
        skipped: 0,
    };

    for alias in source_aliases {
        match dest.get(&alias.alias, &alias.domain).await? {
            Some(existing) => {
                // Check if alias is different
                if alias_matches(&existing, &alias) {
                    // Identical, skip
                    result.skipped += 1;
                } else {
                    // Different, update
                    dest.update(&alias).await?;
                    result.updated += 1;
                }
            }
            None => {
                // Doesn't exist, insert
                dest.put(&alias).await?;
                result.inserted += 1;
            }
        }
    }

    dest.close().await?;

    Ok(result)
}

/// Check if two aliases are identical (excluding modified_at which will differ).
fn alias_matches(a: &Alias, b: &Alias) -> bool {
    a.alias == b.alias
        && a.domain == b.domain
        && a.email_addresses == b.email_addresses
        && a.description == b.description
        && a.suspended == b.suspended
        && a.created_at == b.created_at
        && a.suspended_at == b.suspended_at
}

/// Write a default config.toml to the given directory.
pub fn write_default_config(config_dir: &Path) -> Result<()> {
    std::fs::create_dir_all(config_dir)
        .map_err(|e| Error::Config(format!("failed to create config dir: {}", e)))?;

    let mut systems = std::collections::HashMap::new();
    systems.insert(
        "default".to_string(),
        SystemConfig {
            storage: StorageConfig::Sqlite {
                db_path: config_dir.join("aliasman.db").to_string_lossy().to_string(),
            },
            email: EmailConfig::Rackspace {
                user_key: "your-api-user-key".to_string(),
                secret_key: "your-api-secret-key".to_string(),
            },
            domain: Some("example.com".to_string()),
            email_addresses: Some(vec!["user@example.com".to_string()]),
        },
    );

    let default_config = config::AppConfig {
        default_system: "default".to_string(),
        systems,
    };

    let toml_str = toml::to_string_pretty(&default_config)
        .map_err(|e| Error::Config(format!("failed to serialize config: {}", e)))?;

    let config_file = config_dir.join("config.toml");
    std::fs::write(&config_file, toml_str)
        .map_err(|e| Error::Config(format!("failed to write config file: {}", e)))?;

    Ok(())
}
