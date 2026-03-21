use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::Row;
use uuid::Uuid;

use super::{
    generate_session_token, hash_password, verify_password, Action, AuthError, NewUser, Permission,
    ResourceType, Session, User, UserStore,
};

/// Current schema version, stored in the auth_meta table.
const SCHEMA_VERSION: i32 = 1;

pub struct PostgresUserStore {
    url: String,
    pool: Option<PgPool>,
    session_ttl_hours: u64,
}

impl PostgresUserStore {
    pub fn new(url: &str, session_ttl_hours: u64) -> Self {
        Self {
            url: url.to_string(),
            pool: None,
            session_ttl_hours,
        }
    }

    fn pool(&self) -> Result<&PgPool, AuthError> {
        self.pool
            .as_ref()
            .ok_or_else(|| AuthError::Store("database not opened".into()))
    }

    async fn get_schema_version(pool: &PgPool) -> Result<i32, AuthError> {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                SELECT FROM information_schema.tables
                WHERE table_name = 'auth_meta'
            )",
        )
        .fetch_one(pool)
        .await
        .map_err(|e| AuthError::Store(Box::new(e)))?;

        if !exists {
            return Ok(0);
        }

        let version: i32 =
            sqlx::query_scalar("SELECT value::integer FROM auth_meta WHERE key = 'schema_version'")
                .fetch_optional(pool)
                .await
                .map_err(|e| AuthError::Store(Box::new(e)))?
                .unwrap_or(0);

        Ok(version)
    }

    async fn set_schema_version(pool: &PgPool, version: i32) -> Result<(), AuthError> {
        sqlx::query(
            "INSERT INTO auth_meta (key, value) VALUES ('schema_version', $1)
             ON CONFLICT (key) DO UPDATE SET value = $1",
        )
        .bind(version.to_string())
        .execute(pool)
        .await
        .map_err(|e| AuthError::Store(Box::new(e)))?;

        Ok(())
    }

    async fn migrate(pool: &PgPool, current_version: i32) -> Result<(), AuthError> {
        for version in current_version..SCHEMA_VERSION {
            match version {
                0 => migrate_v0_to_v1(pool).await?,
                _ => {
                    return Err(AuthError::Store(
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

    async fn create_session(&self, user: &User) -> Result<Session, AuthError> {
        let pool = self.pool()?;
        let token = generate_session_token();
        let now = Utc::now();
        let expires_at = now + Duration::hours(self.session_ttl_hours as i64);

        sqlx::query(
            "INSERT INTO auth_sessions (token, user_id, created_at, expires_at)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(&token)
        .bind(&user.id)
        .bind(now)
        .bind(expires_at)
        .execute(pool)
        .await
        .map_err(|e| AuthError::Store(Box::new(e)))?;

        Ok(Session {
            token,
            user_id: user.id.clone(),
            username: user.username.clone(),
            is_superuser: user.is_superuser,
        })
    }
}

/// Migration 0 → 1: initial schema creation.
async fn migrate_v0_to_v1(pool: &PgPool) -> Result<(), AuthError> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS auth_meta (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| AuthError::Store(Box::new(e)))?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS auth_users (
            id            TEXT PRIMARY KEY,
            username      TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            is_superuser  BOOLEAN NOT NULL DEFAULT FALSE,
            created_at    TIMESTAMPTZ NOT NULL,
            updated_at    TIMESTAMPTZ NOT NULL
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| AuthError::Store(Box::new(e)))?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS auth_permissions (
            id            TEXT PRIMARY KEY,
            user_id       TEXT NOT NULL REFERENCES auth_users(id) ON DELETE CASCADE,
            action        TEXT NOT NULL,
            resource_type TEXT NOT NULL,
            resource_id   TEXT,
            UNIQUE(user_id, action, resource_type, resource_id)
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| AuthError::Store(Box::new(e)))?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS auth_sessions (
            token      TEXT PRIMARY KEY,
            user_id    TEXT NOT NULL REFERENCES auth_users(id) ON DELETE CASCADE,
            created_at TIMESTAMPTZ NOT NULL,
            expires_at TIMESTAMPTZ NOT NULL
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| AuthError::Store(Box::new(e)))?;

    Ok(())
}

#[async_trait]
impl UserStore for PostgresUserStore {
    async fn open(&mut self) -> Result<(), AuthError> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&self.url)
            .await
            .map_err(|e| AuthError::Store(Box::new(e)))?;

        let current_version = Self::get_schema_version(&pool).await?;

        if current_version < SCHEMA_VERSION {
            Self::migrate(&pool, current_version).await?;
        } else if current_version > SCHEMA_VERSION {
            return Err(AuthError::Store(
                format!(
                    "user store schema version {} is newer than supported version {}",
                    current_version, SCHEMA_VERSION
                )
                .into(),
            ));
        }

        self.pool = Some(pool);
        Ok(())
    }

    async fn close(&mut self) {
        if let Some(pool) = self.pool.take() {
            pool.close().await;
        }
    }

    async fn create_user(&self, new_user: &NewUser) -> Result<User, AuthError> {
        let pool = self.pool()?;

        let existing = sqlx::query("SELECT id FROM auth_users WHERE username = $1")
            .bind(&new_user.username)
            .fetch_optional(pool)
            .await
            .map_err(|e| AuthError::Store(Box::new(e)))?;

        if existing.is_some() {
            return Err(AuthError::UserAlreadyExists(new_user.username.clone()));
        }

        let id = Uuid::new_v4().to_string();
        let password_hash = hash_password(&new_user.password)?;
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO auth_users (id, username, password_hash, is_superuser, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(&id)
        .bind(&new_user.username)
        .bind(&password_hash)
        .bind(new_user.is_superuser)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .map_err(|e| AuthError::Store(Box::new(e)))?;

        Ok(User {
            id,
            username: new_user.username.clone(),
            is_superuser: new_user.is_superuser,
        })
    }

    async fn get_user(&self, id: &str) -> Result<Option<User>, AuthError> {
        let pool = self.pool()?;
        let row = sqlx::query("SELECT id, username, is_superuser FROM auth_users WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
            .map_err(|e| AuthError::Store(Box::new(e)))?;

        match row {
            Some(row) => Ok(Some(row_to_user(&row)?)),
            None => Ok(None),
        }
    }

    async fn get_user_by_username(&self, username: &str) -> Result<Option<User>, AuthError> {
        let pool = self.pool()?;
        let row =
            sqlx::query("SELECT id, username, is_superuser FROM auth_users WHERE username = $1")
                .bind(username)
                .fetch_optional(pool)
                .await
                .map_err(|e| AuthError::Store(Box::new(e)))?;

        match row {
            Some(row) => Ok(Some(row_to_user(&row)?)),
            None => Ok(None),
        }
    }

    async fn list_users(&self) -> Result<Vec<User>, AuthError> {
        let pool = self.pool()?;
        let rows =
            sqlx::query("SELECT id, username, is_superuser FROM auth_users ORDER BY username")
                .fetch_all(pool)
                .await
                .map_err(|e| AuthError::Store(Box::new(e)))?;

        rows.iter().map(row_to_user).collect()
    }

    async fn delete_user(&self, username: &str) -> Result<(), AuthError> {
        let pool = self.pool()?;
        let result = sqlx::query("DELETE FROM auth_users WHERE username = $1")
            .bind(username)
            .execute(pool)
            .await
            .map_err(|e| AuthError::Store(Box::new(e)))?;

        if result.rows_affected() == 0 {
            return Err(AuthError::UserNotFound(username.to_string()));
        }
        Ok(())
    }

    async fn update_password(&self, username: &str, new_password: &str) -> Result<(), AuthError> {
        let pool = self.pool()?;
        let password_hash = hash_password(new_password)?;
        let now = Utc::now();

        let result = sqlx::query(
            "UPDATE auth_users SET password_hash = $1, updated_at = $2 WHERE username = $3",
        )
        .bind(&password_hash)
        .bind(now)
        .bind(username)
        .execute(pool)
        .await
        .map_err(|e| AuthError::Store(Box::new(e)))?;

        if result.rows_affected() == 0 {
            return Err(AuthError::UserNotFound(username.to_string()));
        }
        Ok(())
    }

    async fn authenticate(&self, username: &str, password: &str) -> Result<Session, AuthError> {
        let pool = self.pool()?;
        let row = sqlx::query(
            "SELECT id, username, password_hash, is_superuser FROM auth_users WHERE username = $1",
        )
        .bind(username)
        .fetch_optional(pool)
        .await
        .map_err(|e| AuthError::Store(Box::new(e)))?;

        let row = row.ok_or(AuthError::InvalidCredentials)?;

        let stored_hash: String = row
            .try_get("password_hash")
            .map_err(|e| AuthError::Store(Box::new(e)))?;

        verify_password(password, &stored_hash)?;

        let user = row_to_user(&row)?;
        self.create_session(&user).await
    }

    async fn get_session(&self, token: &str) -> Result<Session, AuthError> {
        let pool = self.pool()?;
        let row = sqlx::query(
            "SELECT s.token, s.user_id, s.expires_at, u.username, u.is_superuser
             FROM auth_sessions s
             JOIN auth_users u ON u.id = s.user_id
             WHERE s.token = $1",
        )
        .bind(token)
        .fetch_optional(pool)
        .await
        .map_err(|e| AuthError::Store(Box::new(e)))?;

        let row = row.ok_or(AuthError::SessionNotFound)?;

        let expires_at: DateTime<Utc> = row
            .try_get("expires_at")
            .map_err(|e| AuthError::Store(Box::new(e)))?;

        if Utc::now() > expires_at {
            let _ = self.delete_session(token).await;
            return Err(AuthError::SessionNotFound);
        }

        Ok(Session {
            token: row
                .try_get("token")
                .map_err(|e| AuthError::Store(Box::new(e)))?,
            user_id: row
                .try_get("user_id")
                .map_err(|e| AuthError::Store(Box::new(e)))?,
            username: row
                .try_get("username")
                .map_err(|e| AuthError::Store(Box::new(e)))?,
            is_superuser: row
                .try_get("is_superuser")
                .map_err(|e| AuthError::Store(Box::new(e)))?,
        })
    }

    async fn delete_session(&self, token: &str) -> Result<(), AuthError> {
        let pool = self.pool()?;
        sqlx::query("DELETE FROM auth_sessions WHERE token = $1")
            .bind(token)
            .execute(pool)
            .await
            .map_err(|e| AuthError::Store(Box::new(e)))?;
        Ok(())
    }

    async fn cleanup_expired_sessions(&self) -> Result<u64, AuthError> {
        let pool = self.pool()?;
        let now = Utc::now();
        let result = sqlx::query("DELETE FROM auth_sessions WHERE expires_at < $1")
            .bind(now)
            .execute(pool)
            .await
            .map_err(|e| AuthError::Store(Box::new(e)))?;
        Ok(result.rows_affected())
    }

    async fn set_permissions(
        &self,
        user_id: &str,
        permissions: &[Permission],
    ) -> Result<(), AuthError> {
        let pool = self.pool()?;
        for perm in permissions {
            let id = Uuid::new_v4().to_string();
            sqlx::query(
                "INSERT INTO auth_permissions (id, user_id, action, resource_type, resource_id)
                 VALUES ($1, $2, $3, $4, $5)
                 ON CONFLICT (user_id, action, resource_type, resource_id) DO NOTHING",
            )
            .bind(&id)
            .bind(user_id)
            .bind(perm.action.as_str())
            .bind(perm.resource_type.as_str())
            .bind(&perm.resource_id)
            .execute(pool)
            .await
            .map_err(|e| AuthError::Store(Box::new(e)))?;
        }
        Ok(())
    }

    async fn get_permissions(&self, user_id: &str) -> Result<Vec<Permission>, AuthError> {
        let pool = self.pool()?;
        let rows = sqlx::query(
            "SELECT id, user_id, action, resource_type, resource_id
             FROM auth_permissions WHERE user_id = $1
             ORDER BY resource_type, resource_id, action",
        )
        .bind(user_id)
        .fetch_all(pool)
        .await
        .map_err(|e| AuthError::Store(Box::new(e)))?;

        rows.iter().map(row_to_permission).collect()
    }

    async fn clear_permissions(
        &self,
        user_id: &str,
        resource_type: &ResourceType,
        resource_id: &str,
    ) -> Result<(), AuthError> {
        let pool = self.pool()?;
        sqlx::query(
            "DELETE FROM auth_permissions
             WHERE user_id = $1 AND resource_type = $2 AND resource_id = $3",
        )
        .bind(user_id)
        .bind(resource_type.as_str())
        .bind(resource_id)
        .execute(pool)
        .await
        .map_err(|e| AuthError::Store(Box::new(e)))?;
        Ok(())
    }

    async fn check_permission(
        &self,
        user_id: &str,
        action: &Action,
        resource_type: &ResourceType,
        resource_id: &str,
    ) -> Result<bool, AuthError> {
        let user = self
            .get_user(user_id)
            .await?
            .ok_or_else(|| AuthError::UserNotFound(user_id.to_string()))?;

        if user.is_superuser {
            return Ok(true);
        }

        let pool = self.pool()?;

        // Check for exact match or wildcard (resource_id IS NULL) in one query
        let row = sqlx::query(
            "SELECT COUNT(*) as cnt FROM auth_permissions
             WHERE user_id = $1 AND action = $2 AND resource_type = $3
               AND (resource_id = $4 OR resource_id IS NULL)",
        )
        .bind(user_id)
        .bind(action.as_str())
        .bind(resource_type.as_str())
        .bind(resource_id)
        .fetch_one(pool)
        .await
        .map_err(|e| AuthError::Store(Box::new(e)))?;

        let count: i64 = row
            .try_get("cnt")
            .map_err(|e| AuthError::Store(Box::new(e)))?;

        Ok(count > 0)
    }
}

fn row_to_user(row: &sqlx::postgres::PgRow) -> Result<User, AuthError> {
    Ok(User {
        id: row
            .try_get("id")
            .map_err(|e| AuthError::Store(Box::new(e)))?,
        username: row
            .try_get("username")
            .map_err(|e| AuthError::Store(Box::new(e)))?,
        is_superuser: row
            .try_get("is_superuser")
            .map_err(|e| AuthError::Store(Box::new(e)))?,
    })
}

fn row_to_permission(row: &sqlx::postgres::PgRow) -> Result<Permission, AuthError> {
    let action_str: String = row
        .try_get("action")
        .map_err(|e| AuthError::Store(Box::new(e)))?;
    let resource_type_str: String = row
        .try_get("resource_type")
        .map_err(|e| AuthError::Store(Box::new(e)))?;

    Ok(Permission {
        id: row
            .try_get("id")
            .map_err(|e| AuthError::Store(Box::new(e)))?,
        user_id: row
            .try_get("user_id")
            .map_err(|e| AuthError::Store(Box::new(e)))?,
        action: Action::parse(&action_str)?,
        resource_type: ResourceType::parse(&resource_type_str)?,
        resource_id: row
            .try_get("resource_id")
            .map_err(|e| AuthError::Store(Box::new(e)))?,
    })
}
