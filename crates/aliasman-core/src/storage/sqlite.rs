use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use sqlx::Row;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use crate::error::{Error, Result};
use crate::model::{Alias, AliasFilter};
use crate::storage::StorageProvider;

pub struct SqliteStorage {
    db_path: PathBuf,
    pool: Option<SqlitePool>,
}

impl SqliteStorage {
    pub fn new(db_path: &Path) -> Self {
        Self {
            db_path: db_path.to_path_buf(),
            pool: None,
        }
    }

    fn pool(&self) -> Result<&SqlitePool> {
        self.pool
            .as_ref()
            .ok_or_else(|| Error::Storage("database not opened".into()))
    }
}

#[async_trait]
impl StorageProvider for SqliteStorage {
    async fn open(&mut self, read_only: bool) -> Result<()> {
        let options = SqliteConnectOptions::from_str(&format!("sqlite:{}", self.db_path.display()))
            .map_err(|e| Error::Storage(Box::new(e)))?
            .create_if_missing(!read_only)
            .read_only(read_only);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await
            .map_err(|e| Error::Storage(Box::new(e)))?;

        if !read_only {
            sqlx::query(
                "CREATE TABLE IF NOT EXISTS alias (
                    alias         TEXT NOT NULL,
                    domain        TEXT NOT NULL,
                    addresses     TEXT NOT NULL DEFAULT '',
                    description   TEXT NOT NULL DEFAULT '',
                    suspended     INTEGER NOT NULL DEFAULT 0,
                    created_ts    TEXT NOT NULL,
                    modified_ts   TEXT NOT NULL,
                    suspended_ts  TEXT,
                    PRIMARY KEY (alias, domain)
                )",
            )
            .execute(&pool)
            .await
            .map_err(|e| Error::Storage(Box::new(e)))?;
        }

        self.pool = Some(pool);
        Ok(())
    }

    async fn close(&mut self) -> Result<()> {
        if let Some(pool) = self.pool.take() {
            pool.close().await;
        }
        Ok(())
    }

    async fn get(&self, alias: &str, domain: &str) -> Result<Option<Alias>> {
        let pool = self.pool()?;
        let row = sqlx::query(
            "SELECT alias, domain, addresses, description, suspended, created_ts, modified_ts, suspended_ts
             FROM alias WHERE alias = ? AND domain = ?",
        )
        .bind(alias)
        .bind(domain)
        .fetch_optional(pool)
        .await
        .map_err(|e| Error::Storage(Box::new(e)))?;

        match row {
            Some(row) => Ok(Some(row_to_alias(&row)?)),
            None => Ok(None),
        }
    }

    async fn put(&self, alias: &Alias) -> Result<()> {
        let pool = self.pool()?;
        let addresses = alias.email_addresses.join(",");
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO alias (alias, domain, addresses, description, suspended, created_ts, modified_ts, suspended_ts)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&alias.alias)
        .bind(&alias.domain)
        .bind(&addresses)
        .bind(&alias.description)
        .bind(alias.suspended)
        .bind(alias.created_at.to_rfc3339())
        .bind(now.to_rfc3339())
        .bind(alias.suspended_at.map(|t| t.to_rfc3339()))
        .execute(pool)
        .await
        .map_err(|e| Error::Storage(Box::new(e)))?;

        Ok(())
    }

    async fn update(&self, alias: &Alias) -> Result<()> {
        let pool = self.pool()?;
        let addresses = alias.email_addresses.join(",");
        let now = Utc::now();

        let result = sqlx::query(
            "UPDATE alias SET addresses = ?, description = ?, suspended = ?, modified_ts = ?, suspended_ts = ?
             WHERE alias = ? AND domain = ?",
        )
        .bind(&addresses)
        .bind(&alias.description)
        .bind(alias.suspended)
        .bind(now.to_rfc3339())
        .bind(alias.suspended_at.map(|t| t.to_rfc3339()))
        .bind(&alias.alias)
        .bind(&alias.domain)
        .execute(pool)
        .await
        .map_err(|e| Error::Storage(Box::new(e)))?;

        if result.rows_affected() == 0 {
            return Err(Error::AliasNotFound {
                alias: alias.alias.clone(),
                domain: alias.domain.clone(),
            });
        }

        Ok(())
    }

    async fn delete(&self, alias: &str, domain: &str) -> Result<()> {
        let pool = self.pool()?;

        let result = sqlx::query("DELETE FROM alias WHERE alias = ? AND domain = ?")
            .bind(alias)
            .bind(domain)
            .execute(pool)
            .await
            .map_err(|e| Error::Storage(Box::new(e)))?;

        if result.rows_affected() == 0 {
            return Err(Error::AliasNotFound {
                alias: alias.to_string(),
                domain: domain.to_string(),
            });
        }

        Ok(())
    }

    async fn search(&self, filter: &AliasFilter) -> Result<Vec<Alias>> {
        let pool = self.pool()?;

        let rows = sqlx::query(
            "SELECT alias, domain, addresses, description, suspended, created_ts, modified_ts, suspended_ts
             FROM alias ORDER BY domain, alias",
        )
        .fetch_all(pool)
        .await
        .map_err(|e| Error::Storage(Box::new(e)))?;

        let mut aliases = Vec::new();
        for row in rows {
            let alias = row_to_alias(&row)?;
            if alias.matches(filter) {
                aliases.push(alias);
            }
        }

        Ok(aliases)
    }

    async fn suspend(&self, alias: &str, domain: &str) -> Result<()> {
        let pool = self.pool()?;
        let now = Utc::now();

        let result = sqlx::query(
            "UPDATE alias SET suspended = 1, modified_ts = ?, suspended_ts = ? WHERE alias = ? AND domain = ?",
        )
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .bind(alias)
        .bind(domain)
        .execute(pool)
        .await
        .map_err(|e| Error::Storage(Box::new(e)))?;

        if result.rows_affected() == 0 {
            return Err(Error::AliasNotFound {
                alias: alias.to_string(),
                domain: domain.to_string(),
            });
        }

        Ok(())
    }

    async fn unsuspend(&self, alias: &str, domain: &str) -> Result<()> {
        let pool = self.pool()?;
        let now = Utc::now();

        let result = sqlx::query(
            "UPDATE alias SET suspended = 0, modified_ts = ?, suspended_ts = NULL WHERE alias = ? AND domain = ?",
        )
        .bind(now.to_rfc3339())
        .bind(alias)
        .bind(domain)
        .execute(pool)
        .await
        .map_err(|e| Error::Storage(Box::new(e)))?;

        if result.rows_affected() == 0 {
            return Err(Error::AliasNotFound {
                alias: alias.to_string(),
                domain: domain.to_string(),
            });
        }

        Ok(())
    }
}

fn row_to_alias(row: &sqlx::sqlite::SqliteRow) -> Result<Alias> {
    let addresses: String = row
        .try_get("addresses")
        .map_err(|e| Error::Storage(Box::new(e)))?;
    let email_addresses: Vec<String> = if addresses.is_empty() {
        Vec::new()
    } else {
        addresses.split(',').map(|s| s.trim().to_string()).collect()
    };

    let suspended: bool = row
        .try_get("suspended")
        .map_err(|e| Error::Storage(Box::new(e)))?;

    let created_ts: String = row
        .try_get("created_ts")
        .map_err(|e| Error::Storage(Box::new(e)))?;
    let modified_ts: String = row
        .try_get("modified_ts")
        .map_err(|e| Error::Storage(Box::new(e)))?;
    let suspended_ts: Option<String> = row
        .try_get("suspended_ts")
        .map_err(|e| Error::Storage(Box::new(e)))?;

    Ok(Alias {
        alias: row
            .try_get("alias")
            .map_err(|e| Error::Storage(Box::new(e)))?,
        domain: row
            .try_get("domain")
            .map_err(|e| Error::Storage(Box::new(e)))?,
        email_addresses,
        description: row
            .try_get("description")
            .map_err(|e| Error::Storage(Box::new(e)))?,
        suspended,
        created_at: parse_datetime(&created_ts)?,
        modified_at: parse_datetime(&modified_ts)?,
        suspended_at: suspended_ts.as_deref().map(parse_datetime).transpose()?,
    })
}

fn parse_datetime(s: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| Error::Storage(format!("invalid datetime '{}': {}", s, e).into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::AliasFilter;

    async fn setup_storage() -> SqliteStorage {
        let mut storage = SqliteStorage::new(Path::new(":memory:"));
        storage.open(false).await.unwrap();
        storage
    }

    fn sample_alias() -> Alias {
        Alias {
            alias: "test123".to_string(),
            domain: "example.com".to_string(),
            email_addresses: vec!["user@example.com".to_string()],
            description: "Test alias".to_string(),
            suspended: false,
            created_at: Utc::now(),
            modified_at: Utc::now(),
            suspended_at: None,
        }
    }

    #[tokio::test]
    async fn test_put_and_get() {
        let storage = setup_storage().await;
        let alias = sample_alias();

        storage.put(&alias).await.unwrap();
        let fetched = storage.get("test123", "example.com").await.unwrap();
        assert!(fetched.is_some());

        let fetched = fetched.unwrap();
        assert_eq!(fetched.alias, "test123");
        assert_eq!(fetched.domain, "example.com");
        assert_eq!(fetched.email_addresses, vec!["user@example.com"]);
        assert_eq!(fetched.description, "Test alias");
        assert!(!fetched.suspended);
    }

    #[tokio::test]
    async fn test_get_nonexistent() {
        let storage = setup_storage().await;
        let fetched = storage.get("nope", "nope.com").await.unwrap();
        assert!(fetched.is_none());
    }

    #[tokio::test]
    async fn test_delete() {
        let storage = setup_storage().await;
        storage.put(&sample_alias()).await.unwrap();
        storage.delete("test123", "example.com").await.unwrap();
        let fetched = storage.get("test123", "example.com").await.unwrap();
        assert!(fetched.is_none());
    }

    #[tokio::test]
    async fn test_delete_nonexistent() {
        let storage = setup_storage().await;
        let result = storage.delete("nope", "nope.com").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_search_all() {
        let storage = setup_storage().await;
        storage.put(&sample_alias()).await.unwrap();

        let mut alias2 = sample_alias();
        alias2.alias = "other".to_string();
        alias2.description = "Other alias".to_string();
        storage.put(&alias2).await.unwrap();

        let results = storage.search(&AliasFilter::default()).await.unwrap();
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_suspend_and_unsuspend() {
        let storage = setup_storage().await;
        storage.put(&sample_alias()).await.unwrap();

        storage.suspend("test123", "example.com").await.unwrap();
        let fetched = storage
            .get("test123", "example.com")
            .await
            .unwrap()
            .unwrap();
        assert!(fetched.suspended);
        assert!(fetched.suspended_at.is_some());

        storage.unsuspend("test123", "example.com").await.unwrap();
        let fetched = storage
            .get("test123", "example.com")
            .await
            .unwrap()
            .unwrap();
        assert!(!fetched.suspended);
        assert!(fetched.suspended_at.is_none());
    }

    #[tokio::test]
    async fn test_update() {
        let storage = setup_storage().await;
        storage.put(&sample_alias()).await.unwrap();

        let mut alias = sample_alias();
        alias.description = "Updated".to_string();
        alias.email_addresses = vec!["new@example.com".to_string()];
        storage.update(&alias).await.unwrap();

        let fetched = storage
            .get("test123", "example.com")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(fetched.description, "Updated");
        assert_eq!(fetched.email_addresses, vec!["new@example.com"]);
    }
}
