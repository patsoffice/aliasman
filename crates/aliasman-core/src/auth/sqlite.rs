use async_trait::async_trait;
use chrono::{Duration, Utc};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use sqlx::Row;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use uuid::Uuid;

use super::{
    generate_session_token, hash_password, verify_password, Action, AuthError, NewUser, Permission,
    ResourceType, Session, User, UserStore,
};

/// Current schema version for the user store database.
const SCHEMA_VERSION: u32 = 1;

pub struct SqliteUserStore {
    db_path: PathBuf,
    pool: Option<SqlitePool>,
    session_ttl_hours: u64,
}

impl SqliteUserStore {
    pub fn new(db_path: &Path, session_ttl_hours: u64) -> Self {
        Self {
            db_path: db_path.to_path_buf(),
            pool: None,
            session_ttl_hours,
        }
    }

    fn pool(&self) -> Result<&SqlitePool, AuthError> {
        self.pool
            .as_ref()
            .ok_or_else(|| AuthError::Store("database not opened".into()))
    }

    async fn migrate(pool: &SqlitePool, current_version: u32) -> Result<(), AuthError> {
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

        sqlx::query(&format!("PRAGMA user_version = {SCHEMA_VERSION}"))
            .execute(pool)
            .await
            .map_err(|e| AuthError::Store(Box::new(e)))?;

        Ok(())
    }

    async fn create_session(&self, user: &User) -> Result<Session, AuthError> {
        let pool = self.pool()?;
        let token = generate_session_token();
        let now = Utc::now();
        let expires_at = now + Duration::hours(self.session_ttl_hours as i64);

        sqlx::query(
            "INSERT INTO sessions (token, user_id, created_at, expires_at) VALUES (?, ?, ?, ?)",
        )
        .bind(&token)
        .bind(&user.id)
        .bind(now.to_rfc3339())
        .bind(expires_at.to_rfc3339())
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
async fn migrate_v0_to_v1(pool: &SqlitePool) -> Result<(), AuthError> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS users (
            id            TEXT PRIMARY KEY,
            username      TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            is_superuser  INTEGER NOT NULL DEFAULT 0,
            created_at    TEXT NOT NULL,
            updated_at    TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| AuthError::Store(Box::new(e)))?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS permissions (
            id            TEXT PRIMARY KEY,
            user_id       TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
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
        "CREATE TABLE IF NOT EXISTS sessions (
            token      TEXT PRIMARY KEY,
            user_id    TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            created_at TEXT NOT NULL,
            expires_at TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| AuthError::Store(Box::new(e)))?;

    Ok(())
}

#[async_trait]
impl UserStore for SqliteUserStore {
    async fn open(&mut self) -> Result<(), AuthError> {
        let options = SqliteConnectOptions::from_str(&format!("sqlite:{}", self.db_path.display()))
            .map_err(|e| AuthError::Store(Box::new(e)))?
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await
            .map_err(|e| AuthError::Store(Box::new(e)))?;

        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await
            .map_err(|e| AuthError::Store(Box::new(e)))?;

        let row = sqlx::query("PRAGMA user_version")
            .fetch_one(&pool)
            .await
            .map_err(|e| AuthError::Store(Box::new(e)))?;
        let current_version: u32 = row
            .try_get::<u32, _>(0)
            .map_err(|e| AuthError::Store(Box::new(e)))?;

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

        let existing = sqlx::query("SELECT id FROM users WHERE username = ?")
            .bind(&new_user.username)
            .fetch_optional(pool)
            .await
            .map_err(|e| AuthError::Store(Box::new(e)))?;

        if existing.is_some() {
            return Err(AuthError::UserAlreadyExists(new_user.username.clone()));
        }

        let id = Uuid::new_v4().to_string();
        let password_hash = hash_password(&new_user.password)?;
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO users (id, username, password_hash, is_superuser, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(&new_user.username)
        .bind(&password_hash)
        .bind(new_user.is_superuser)
        .bind(&now)
        .bind(&now)
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
        let row = sqlx::query("SELECT id, username, is_superuser FROM users WHERE id = ?")
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
        let row = sqlx::query("SELECT id, username, is_superuser FROM users WHERE username = ?")
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
        let rows = sqlx::query("SELECT id, username, is_superuser FROM users ORDER BY username")
            .fetch_all(pool)
            .await
            .map_err(|e| AuthError::Store(Box::new(e)))?;

        rows.iter().map(row_to_user).collect()
    }

    async fn delete_user(&self, username: &str) -> Result<(), AuthError> {
        let pool = self.pool()?;
        let result = sqlx::query("DELETE FROM users WHERE username = ?")
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
        let now = Utc::now().to_rfc3339();

        let result =
            sqlx::query("UPDATE users SET password_hash = ?, updated_at = ? WHERE username = ?")
                .bind(&password_hash)
                .bind(&now)
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
            "SELECT id, username, password_hash, is_superuser FROM users WHERE username = ?",
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
             FROM sessions s
             JOIN users u ON u.id = s.user_id
             WHERE s.token = ?",
        )
        .bind(token)
        .fetch_optional(pool)
        .await
        .map_err(|e| AuthError::Store(Box::new(e)))?;

        let row = row.ok_or(AuthError::SessionNotFound)?;

        let expires_at: String = row
            .try_get("expires_at")
            .map_err(|e| AuthError::Store(Box::new(e)))?;
        let expires_at = chrono::DateTime::parse_from_rfc3339(&expires_at)
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
        sqlx::query("DELETE FROM sessions WHERE token = ?")
            .bind(token)
            .execute(pool)
            .await
            .map_err(|e| AuthError::Store(Box::new(e)))?;
        Ok(())
    }

    async fn cleanup_expired_sessions(&self) -> Result<u64, AuthError> {
        let pool = self.pool()?;
        let now = Utc::now().to_rfc3339();
        let result = sqlx::query("DELETE FROM sessions WHERE expires_at < ?")
            .bind(&now)
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
                "INSERT OR IGNORE INTO permissions (id, user_id, action, resource_type, resource_id)
                 VALUES (?, ?, ?, ?, ?)",
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
             FROM permissions WHERE user_id = ?
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
            "DELETE FROM permissions WHERE user_id = ? AND resource_type = ? AND resource_id = ?",
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

        // Check for exact permission match
        let row = sqlx::query(
            "SELECT COUNT(*) as cnt FROM permissions
             WHERE user_id = ? AND action = ? AND resource_type = ? AND resource_id = ?",
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

        if count > 0 {
            return Ok(true);
        }

        // Check for wildcard permission (resource_id IS NULL)
        let row = sqlx::query(
            "SELECT COUNT(*) as cnt FROM permissions
             WHERE user_id = ? AND action = ? AND resource_type = ? AND resource_id IS NULL",
        )
        .bind(user_id)
        .bind(action.as_str())
        .bind(resource_type.as_str())
        .fetch_one(pool)
        .await
        .map_err(|e| AuthError::Store(Box::new(e)))?;

        let count: i64 = row
            .try_get("cnt")
            .map_err(|e| AuthError::Store(Box::new(e)))?;

        Ok(count > 0)
    }
}

fn row_to_user(row: &sqlx::sqlite::SqliteRow) -> Result<User, AuthError> {
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

fn row_to_permission(row: &sqlx::sqlite::SqliteRow) -> Result<Permission, AuthError> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    async fn setup_store() -> SqliteUserStore {
        let mut store = SqliteUserStore::new(Path::new(":memory:"), 24);
        store.open().await.unwrap();
        store
    }

    fn new_user(username: &str, is_superuser: bool) -> NewUser {
        NewUser {
            username: username.to_string(),
            password: "testpassword123".to_string(),
            is_superuser,
        }
    }

    #[tokio::test]
    async fn test_create_and_get_user() {
        let store = setup_store().await;
        let user = store.create_user(&new_user("alice", false)).await.unwrap();
        assert_eq!(user.username, "alice");
        assert!(!user.is_superuser);

        let fetched = store.get_user_by_username("alice").await.unwrap().unwrap();
        assert_eq!(fetched.id, user.id);
    }

    #[tokio::test]
    async fn test_create_duplicate_user() {
        let store = setup_store().await;
        store.create_user(&new_user("alice", false)).await.unwrap();
        let result = store.create_user(&new_user("alice", false)).await;
        assert!(matches!(result, Err(AuthError::UserAlreadyExists(_))));
    }

    #[tokio::test]
    async fn test_create_superuser() {
        let store = setup_store().await;
        let user = store.create_user(&new_user("admin", true)).await.unwrap();
        assert!(user.is_superuser);
    }

    #[tokio::test]
    async fn test_list_users() {
        let store = setup_store().await;
        store.create_user(&new_user("bob", false)).await.unwrap();
        store.create_user(&new_user("alice", false)).await.unwrap();

        let users = store.list_users().await.unwrap();
        assert_eq!(users.len(), 2);
        assert_eq!(users[0].username, "alice");
        assert_eq!(users[1].username, "bob");
    }

    #[tokio::test]
    async fn test_delete_user() {
        let store = setup_store().await;
        store.create_user(&new_user("alice", false)).await.unwrap();
        store.delete_user("alice").await.unwrap();

        let fetched = store.get_user_by_username("alice").await.unwrap();
        assert!(fetched.is_none());
    }

    #[tokio::test]
    async fn test_delete_nonexistent_user() {
        let store = setup_store().await;
        let result = store.delete_user("nobody").await;
        assert!(matches!(result, Err(AuthError::UserNotFound(_))));
    }

    #[tokio::test]
    async fn test_update_password() {
        let store = setup_store().await;
        store.create_user(&new_user("alice", false)).await.unwrap();
        store
            .update_password("alice", "newpassword456")
            .await
            .unwrap();

        let result = store.authenticate("alice", "testpassword123").await;
        assert!(matches!(result, Err(AuthError::InvalidCredentials)));

        let session = store.authenticate("alice", "newpassword456").await.unwrap();
        assert_eq!(session.username, "alice");
    }

    #[tokio::test]
    async fn test_authenticate_success() {
        let store = setup_store().await;
        store.create_user(&new_user("alice", false)).await.unwrap();

        let session = store
            .authenticate("alice", "testpassword123")
            .await
            .unwrap();
        assert_eq!(session.username, "alice");
        assert!(!session.is_superuser);
        assert!(!session.token.is_empty());
    }

    #[tokio::test]
    async fn test_authenticate_wrong_password() {
        let store = setup_store().await;
        store.create_user(&new_user("alice", false)).await.unwrap();

        let result = store.authenticate("alice", "wrongpassword").await;
        assert!(matches!(result, Err(AuthError::InvalidCredentials)));
    }

    #[tokio::test]
    async fn test_authenticate_nonexistent_user() {
        let store = setup_store().await;
        let result = store.authenticate("nobody", "password").await;
        assert!(matches!(result, Err(AuthError::InvalidCredentials)));
    }

    #[tokio::test]
    async fn test_session_lifecycle() {
        let store = setup_store().await;
        store.create_user(&new_user("alice", false)).await.unwrap();

        let session = store
            .authenticate("alice", "testpassword123")
            .await
            .unwrap();

        let fetched = store.get_session(&session.token).await.unwrap();
        assert_eq!(fetched.username, "alice");

        store.delete_session(&session.token).await.unwrap();
        let result = store.get_session(&session.token).await;
        assert!(matches!(result, Err(AuthError::SessionNotFound)));
    }

    #[tokio::test]
    async fn test_delete_user_cascades_sessions() {
        let store = setup_store().await;
        store.create_user(&new_user("alice", false)).await.unwrap();
        let session = store
            .authenticate("alice", "testpassword123")
            .await
            .unwrap();

        store.delete_user("alice").await.unwrap();

        let result = store.get_session(&session.token).await;
        assert!(matches!(result, Err(AuthError::SessionNotFound)));
    }

    #[tokio::test]
    async fn test_permissions_crud() {
        let store = setup_store().await;
        let user = store.create_user(&new_user("alice", false)).await.unwrap();

        let perms = vec![
            Permission {
                id: String::new(),
                user_id: user.id.clone(),
                action: Action::View,
                resource_type: ResourceType::Domain,
                resource_id: Some("example.com".to_string()),
            },
            Permission {
                id: String::new(),
                user_id: user.id.clone(),
                action: Action::Create,
                resource_type: ResourceType::Domain,
                resource_id: Some("example.com".to_string()),
            },
        ];

        store.set_permissions(&user.id, &perms).await.unwrap();

        let fetched = store.get_permissions(&user.id).await.unwrap();
        assert_eq!(fetched.len(), 2);

        store
            .clear_permissions(&user.id, &ResourceType::Domain, "example.com")
            .await
            .unwrap();
        let fetched = store.get_permissions(&user.id).await.unwrap();
        assert_eq!(fetched.len(), 0);
    }

    #[tokio::test]
    async fn test_check_permission_superuser() {
        let store = setup_store().await;
        let user = store.create_user(&new_user("admin", true)).await.unwrap();

        let allowed = store
            .check_permission(&user.id, &Action::Delete, &ResourceType::Domain, "any.com")
            .await
            .unwrap();
        assert!(allowed);
    }

    #[tokio::test]
    async fn test_check_permission_domain_grant() {
        let store = setup_store().await;
        let user = store.create_user(&new_user("alice", false)).await.unwrap();

        let perms = vec![Permission {
            id: String::new(),
            user_id: user.id.clone(),
            action: Action::View,
            resource_type: ResourceType::Domain,
            resource_id: Some("example.com".to_string()),
        }];
        store.set_permissions(&user.id, &perms).await.unwrap();

        assert!(store
            .check_permission(
                &user.id,
                &Action::View,
                &ResourceType::Domain,
                "example.com"
            )
            .await
            .unwrap());

        assert!(!store
            .check_permission(
                &user.id,
                &Action::Delete,
                &ResourceType::Domain,
                "example.com"
            )
            .await
            .unwrap());

        assert!(!store
            .check_permission(&user.id, &Action::View, &ResourceType::Domain, "other.com")
            .await
            .unwrap());
    }

    #[tokio::test]
    async fn test_check_permission_system_grant() {
        let store = setup_store().await;
        let user = store.create_user(&new_user("alice", false)).await.unwrap();

        let perms: Vec<Permission> = Action::all()
            .iter()
            .map(|action| Permission {
                id: String::new(),
                user_id: user.id.clone(),
                action: action.clone(),
                resource_type: ResourceType::System,
                resource_id: Some("home".to_string()),
            })
            .collect();
        store.set_permissions(&user.id, &perms).await.unwrap();

        assert!(store
            .check_permission(&user.id, &Action::View, &ResourceType::System, "home")
            .await
            .unwrap());

        assert!(!store
            .check_permission(&user.id, &Action::View, &ResourceType::System, "work")
            .await
            .unwrap());
    }

    #[tokio::test]
    async fn test_check_permission_no_grants() {
        let store = setup_store().await;
        let user = store.create_user(&new_user("alice", false)).await.unwrap();

        let allowed = store
            .check_permission(
                &user.id,
                &Action::View,
                &ResourceType::Domain,
                "example.com",
            )
            .await
            .unwrap();
        assert!(!allowed);
    }

    #[tokio::test]
    async fn test_delete_user_cascades_permissions() {
        let store = setup_store().await;
        let user = store.create_user(&new_user("alice", false)).await.unwrap();

        let perms = vec![Permission {
            id: String::new(),
            user_id: user.id.clone(),
            action: Action::View,
            resource_type: ResourceType::Domain,
            resource_id: Some("example.com".to_string()),
        }];
        store.set_permissions(&user.id, &perms).await.unwrap();

        store.delete_user("alice").await.unwrap();

        let pool = store.pool().unwrap();
        let row = sqlx::query("SELECT COUNT(*) as cnt FROM permissions WHERE user_id = ?")
            .bind(&user.id)
            .fetch_one(pool)
            .await
            .unwrap();
        let count: i64 = row.try_get("cnt").unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_schema_version() {
        let store = setup_store().await;
        let pool = store.pool().unwrap();

        let row = sqlx::query("PRAGMA user_version")
            .fetch_one(pool)
            .await
            .unwrap();
        let version: u32 = row.try_get::<u32, _>(0).unwrap();
        assert_eq!(version, SCHEMA_VERSION);
    }

    #[tokio::test]
    async fn test_duplicate_permission_ignored() {
        let store = setup_store().await;
        let user = store.create_user(&new_user("alice", false)).await.unwrap();

        let perm = Permission {
            id: String::new(),
            user_id: user.id.clone(),
            action: Action::View,
            resource_type: ResourceType::Domain,
            resource_id: Some("example.com".to_string()),
        };

        store
            .set_permissions(&user.id, std::slice::from_ref(&perm))
            .await
            .unwrap();
        store
            .set_permissions(&user.id, std::slice::from_ref(&perm))
            .await
            .unwrap();

        let fetched = store.get_permissions(&user.id).await.unwrap();
        assert_eq!(fetched.len(), 1);
    }
}
