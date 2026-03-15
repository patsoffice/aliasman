use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::Row;

use crate::error::{Error, Result};
use crate::model::{Alias, AliasFilter};
use crate::storage::StorageProvider;

/// Current schema version, stored in the aliasman_meta table.
const SCHEMA_VERSION: i32 = 1;

pub struct PostgresStorage {
    url: String,
    pool: Option<PgPool>,
}

impl PostgresStorage {
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            pool: None,
        }
    }

    fn pool(&self) -> Result<&PgPool> {
        self.pool
            .as_ref()
            .ok_or_else(|| Error::Storage("database not opened".into()))
    }

    async fn get_schema_version(pool: &PgPool) -> Result<i32> {
        // Check if meta table exists
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                SELECT FROM information_schema.tables
                WHERE table_name = 'aliasman_meta'
            )",
        )
        .fetch_one(pool)
        .await
        .map_err(|e| Error::Storage(Box::new(e)))?;

        if !exists {
            return Ok(0);
        }

        let version: i32 = sqlx::query_scalar(
            "SELECT value::integer FROM aliasman_meta WHERE key = 'schema_version'",
        )
        .fetch_optional(pool)
        .await
        .map_err(|e| Error::Storage(Box::new(e)))?
        .unwrap_or(0);

        Ok(version)
    }

    async fn set_schema_version(pool: &PgPool, version: i32) -> Result<()> {
        sqlx::query(
            "INSERT INTO aliasman_meta (key, value) VALUES ('schema_version', $1)
             ON CONFLICT (key) DO UPDATE SET value = $1",
        )
        .bind(version.to_string())
        .execute(pool)
        .await
        .map_err(|e| Error::Storage(Box::new(e)))?;

        Ok(())
    }

    async fn migrate(pool: &PgPool, current_version: i32) -> Result<()> {
        for version in current_version..SCHEMA_VERSION {
            match version {
                0 => migrate_v0_to_v1(pool).await?,
                _ => {
                    return Err(Error::Storage(
                        format!(
                            "unknown migration from version {} to {}",
                            version,
                            version + 1
                        )
                        .into(),
                    ));
                }
            }
        }

        Self::set_schema_version(pool, SCHEMA_VERSION).await?;

        Ok(())
    }
}

/// Migration 0 → 1: initial schema creation.
async fn migrate_v0_to_v1(pool: &PgPool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS aliasman_meta (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| Error::Storage(Box::new(e)))?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS alias (
            alias         TEXT NOT NULL,
            domain        TEXT NOT NULL,
            addresses     TEXT NOT NULL DEFAULT '',
            description   TEXT NOT NULL DEFAULT '',
            suspended     BOOLEAN NOT NULL DEFAULT FALSE,
            created_ts    TIMESTAMPTZ NOT NULL,
            modified_ts   TIMESTAMPTZ NOT NULL,
            suspended_ts  TIMESTAMPTZ,
            PRIMARY KEY (alias, domain)
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| Error::Storage(Box::new(e)))?;

    Ok(())
}

#[async_trait]
impl StorageProvider for PostgresStorage {
    async fn open(&mut self, read_only: bool) -> Result<()> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&self.url)
            .await
            .map_err(|e| Error::Storage(Box::new(e)))?;

        if !read_only {
            let current_version = Self::get_schema_version(&pool).await?;

            if current_version < SCHEMA_VERSION {
                Self::migrate(&pool, current_version).await?;
            } else if current_version > SCHEMA_VERSION {
                return Err(Error::Storage(
                    format!(
                        "database schema version {} is newer than supported version {}",
                        current_version, SCHEMA_VERSION
                    )
                    .into(),
                ));
            }
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
             FROM alias WHERE alias = $1 AND domain = $2",
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

        sqlx::query(
            "INSERT INTO alias (alias, domain, addresses, description, suspended, created_ts, modified_ts, suspended_ts)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(&alias.alias)
        .bind(&alias.domain)
        .bind(&addresses)
        .bind(&alias.description)
        .bind(alias.suspended)
        .bind(alias.created_at)
        .bind(alias.modified_at)
        .bind(alias.suspended_at)
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
            "UPDATE alias SET addresses = $1, description = $2, suspended = $3, modified_ts = $4, suspended_ts = $5
             WHERE alias = $6 AND domain = $7",
        )
        .bind(&addresses)
        .bind(&alias.description)
        .bind(alias.suspended)
        .bind(now)
        .bind(alias.suspended_at)
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

        let result = sqlx::query("DELETE FROM alias WHERE alias = $1 AND domain = $2")
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
            "UPDATE alias SET suspended = TRUE, modified_ts = $1, suspended_ts = $2 WHERE alias = $3 AND domain = $4",
        )
        .bind(now)
        .bind(now)
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
            "UPDATE alias SET suspended = FALSE, modified_ts = $1, suspended_ts = NULL WHERE alias = $2 AND domain = $3",
        )
        .bind(now)
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

fn row_to_alias(row: &sqlx::postgres::PgRow) -> Result<Alias> {
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

    let created_at: DateTime<Utc> = row
        .try_get("created_ts")
        .map_err(|e| Error::Storage(Box::new(e)))?;
    let modified_at: DateTime<Utc> = row
        .try_get("modified_ts")
        .map_err(|e| Error::Storage(Box::new(e)))?;
    let suspended_at: Option<DateTime<Utc>> = row
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
        created_at,
        modified_at,
        suspended_at,
    })
}
