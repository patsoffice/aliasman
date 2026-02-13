pub mod s3;
pub mod sqlite;

use async_trait::async_trait;

use crate::error::Result;
use crate::model::{Alias, AliasFilter};

#[async_trait]
pub trait StorageProvider: Send + Sync {
    /// Open the storage backend. If `read_only` is true, no writes are permitted.
    async fn open(&mut self, read_only: bool) -> Result<()>;

    /// Close the storage backend, flushing any pending writes.
    async fn close(&mut self) -> Result<()>;

    /// Get a single alias by its alias name and domain.
    async fn get(&self, alias: &str, domain: &str) -> Result<Option<Alias>>;

    /// Insert a new alias. Returns an error if the alias already exists.
    async fn put(&self, alias: &Alias) -> Result<()>;

    /// Update an existing alias. Returns an error if the alias does not exist.
    async fn update(&self, alias: &Alias) -> Result<()>;

    /// Delete an alias by its alias name and domain.
    async fn delete(&self, alias: &str, domain: &str) -> Result<()>;

    /// Search for aliases matching the given filter.
    async fn search(&self, filter: &AliasFilter) -> Result<Vec<Alias>>;

    /// Suspend an alias (mark as suspended in storage).
    async fn suspend(&self, alias: &str, domain: &str) -> Result<()>;

    /// Unsuspend an alias (mark as active in storage).
    async fn unsuspend(&self, alias: &str, domain: &str) -> Result<()>;

    /// Refresh the storage provider's data from the underlying backend.
    /// No-op for backends that query on every access (e.g., SQLite).
    async fn refresh(&mut self) -> Result<()> {
        Ok(())
    }
}
