pub mod postgres;
pub mod sqlite;

use async_trait::async_trait;
use thiserror::Error;

/// Actions that can be performed on resources.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Action {
    View,
    Create,
    Delete,
    Suspend,
    Unsuspend,
}

impl Action {
    /// All available actions (useful for granting full access).
    pub fn all() -> &'static [Action] {
        &[
            Action::View,
            Action::Create,
            Action::Delete,
            Action::Suspend,
            Action::Unsuspend,
        ]
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Action::View => "view",
            Action::Create => "create",
            Action::Delete => "delete",
            Action::Suspend => "suspend",
            Action::Unsuspend => "unsuspend",
        }
    }

    pub fn parse(s: &str) -> Result<Self, AuthError> {
        match s {
            "view" => Ok(Action::View),
            "create" => Ok(Action::Create),
            "delete" => Ok(Action::Delete),
            "suspend" => Ok(Action::Suspend),
            "unsuspend" => Ok(Action::Unsuspend),
            _ => Err(AuthError::InvalidInput(format!("unknown action: {}", s))),
        }
    }
}

/// Types of resources that permissions can be scoped to.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ResourceType {
    System,
    Domain,
}

impl ResourceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ResourceType::System => "system",
            ResourceType::Domain => "domain",
        }
    }

    pub fn parse(s: &str) -> Result<Self, AuthError> {
        match s {
            "system" => Ok(ResourceType::System),
            "domain" => Ok(ResourceType::Domain),
            _ => Err(AuthError::InvalidInput(format!(
                "unknown resource type: {}",
                s
            ))),
        }
    }
}

/// A permission grant: allows a specific action on a specific resource.
#[derive(Debug, Clone)]
pub struct Permission {
    pub id: String,
    pub user_id: String,
    pub action: Action,
    pub resource_type: ResourceType,
    /// The resource identifier (e.g., system name, domain name).
    /// None means "all resources of this type" (wildcard).
    pub resource_id: Option<String>,
}

/// An authenticated user session.
#[derive(Debug, Clone)]
pub struct Session {
    pub token: String,
    pub user_id: String,
    pub username: String,
    pub is_superuser: bool,
}

/// A user account.
#[derive(Debug, Clone)]
pub struct User {
    pub id: String,
    pub username: String,
    pub is_superuser: bool,
}

/// Data for creating a new user.
pub struct NewUser {
    pub username: String,
    pub password: String,
    pub is_superuser: bool,
}

/// Auth-specific error type.
#[derive(Debug, Error)]
pub enum AuthError {
    #[error("invalid credentials")]
    InvalidCredentials,

    #[error("user not found: {0}")]
    UserNotFound(String),

    #[error("user already exists: {0}")]
    UserAlreadyExists(String),

    #[error("permission denied")]
    PermissionDenied,

    #[error("session not found or expired")]
    SessionNotFound,

    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("auth store error: {0}")]
    Store(#[source] Box<dyn std::error::Error + Send + Sync>),
}

/// User store backend trait.
///
/// Provides user management, authentication, session management,
/// and permission checking. Implementations exist for SQLite and PostgreSQL.
#[async_trait]
pub trait UserStore: Send + Sync {
    async fn open(&mut self) -> Result<(), AuthError>;
    async fn close(&mut self);

    // User operations
    async fn create_user(&self, new_user: &NewUser) -> Result<User, AuthError>;
    async fn get_user(&self, id: &str) -> Result<Option<User>, AuthError>;
    async fn get_user_by_username(&self, username: &str) -> Result<Option<User>, AuthError>;
    async fn list_users(&self) -> Result<Vec<User>, AuthError>;
    async fn delete_user(&self, username: &str) -> Result<(), AuthError>;
    async fn update_password(&self, username: &str, new_password: &str) -> Result<(), AuthError>;

    // Authentication
    async fn authenticate(&self, username: &str, password: &str) -> Result<Session, AuthError>;

    // Session operations
    async fn get_session(&self, token: &str) -> Result<Session, AuthError>;
    async fn delete_session(&self, token: &str) -> Result<(), AuthError>;
    async fn cleanup_expired_sessions(&self) -> Result<u64, AuthError>;

    // Permission operations
    async fn set_permissions(
        &self,
        user_id: &str,
        permissions: &[Permission],
    ) -> Result<(), AuthError>;
    async fn get_permissions(&self, user_id: &str) -> Result<Vec<Permission>, AuthError>;
    async fn clear_permissions(
        &self,
        user_id: &str,
        resource_type: &ResourceType,
        resource_id: &str,
    ) -> Result<(), AuthError>;

    /// Check if a user has permission for an action on a resource.
    ///
    /// Resolution order:
    /// 1. Superuser → always allowed
    /// 2. Wildcard grant (resource_id IS NULL) → allowed for all resources of that type
    /// 3. Exact grant → allowed for that specific resource
    async fn check_permission(
        &self,
        user_id: &str,
        action: &Action,
        resource_type: &ResourceType,
        resource_id: &str,
    ) -> Result<bool, AuthError>;
}

// --- Password hashing helpers (shared by implementations) ---

pub(crate) fn hash_password(password: &str) -> Result<String, AuthError> {
    use argon2::password_hash::rand_core::OsRng;
    use argon2::password_hash::SaltString;
    use argon2::{Argon2, PasswordHasher};

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| AuthError::Store(format!("password hashing failed: {}", e).into()))
}

pub(crate) fn verify_password(password: &str, hash: &str) -> Result<(), AuthError> {
    use argon2::{Argon2, PasswordHash, PasswordVerifier};

    let parsed_hash = PasswordHash::new(hash)
        .map_err(|e| AuthError::Store(format!("invalid hash: {}", e).into()))?;
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .map_err(|_| AuthError::InvalidCredentials)
}

pub(crate) fn generate_session_token() -> String {
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine;
    use rand::RngCore;

    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}
