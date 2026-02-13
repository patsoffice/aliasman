//! Auth/RBAC type definitions and no-op implementation.
//! These are stubs for future authentication and authorization support.

#![allow(dead_code)]

use async_trait::async_trait;

/// Actions that can be performed on resources.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Action {
    View,
    Create,
    Delete,
    Suspend,
    Unsuspend,
    ManageUsers,
    ManageConfig,
}

/// Types of resources that permissions can be scoped to.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ResourceType {
    Global,
    System,
    Domain,
    Alias,
}

/// A specific resource instance.
#[derive(Debug, Clone)]
pub struct Resource {
    pub resource_type: ResourceType,
    /// The resource identifier (e.g., system name, domain name).
    /// None means "all resources of this type" (wildcard).
    pub id: Option<String>,
}

/// A permission grant: allows a specific action on a specific resource.
#[derive(Debug, Clone)]
pub struct Permission {
    pub action: Action,
    pub resource: Resource,
}

/// An authenticated user session.
#[derive(Debug, Clone)]
pub struct Session {
    pub user_id: String,
    pub username: String,
    pub is_superuser: bool,
}

/// Credentials for authentication (placeholder for future auth methods).
#[derive(Debug)]
pub enum Credentials {
    UsernamePassword { username: String, password: String },
}

/// A user account.
#[derive(Debug, Clone)]
pub struct User {
    pub id: String,
    pub username: String,
}

/// Data for creating a new user.
#[derive(Debug)]
pub struct NewUser {
    pub username: String,
    pub password: String,
}

/// Authentication and authorization provider.
#[async_trait]
pub trait AuthProvider: Send + Sync {
    async fn authenticate(&self, credentials: &Credentials) -> Result<Session, AuthError>;
    async fn get_permissions(&self, user_id: &str) -> Result<Vec<Permission>, AuthError>;
    async fn check_permission(
        &self,
        user_id: &str,
        action: &Action,
        resource: &Resource,
    ) -> Result<bool, AuthError>;
}

/// User storage backend.
#[async_trait]
pub trait UserStore: Send + Sync {
    async fn get_user(&self, id: &str) -> Result<Option<User>, AuthError>;
    async fn create_user(&self, user: &NewUser) -> Result<User, AuthError>;
    async fn list_users(&self) -> Result<Vec<User>, AuthError>;
    async fn set_permissions(
        &self,
        user_id: &str,
        permissions: &[Permission],
    ) -> Result<(), AuthError>;
    async fn get_permissions(&self, user_id: &str) -> Result<Vec<Permission>, AuthError>;
}

/// Auth-specific error type.
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("user not found: {0}")]
    UserNotFound(String),
    #[error("permission denied")]
    PermissionDenied,
    #[error("internal auth error: {0}")]
    Internal(String),
}

/// No-op auth provider that grants superuser access to all requests.
/// Used as the default when auth is not configured.
pub struct NoAuthProvider;

#[async_trait]
impl AuthProvider for NoAuthProvider {
    async fn authenticate(&self, _credentials: &Credentials) -> Result<Session, AuthError> {
        Ok(Session {
            user_id: "anonymous".to_string(),
            username: "anonymous".to_string(),
            is_superuser: true,
        })
    }

    async fn get_permissions(&self, _user_id: &str) -> Result<Vec<Permission>, AuthError> {
        Ok(vec![])
    }

    async fn check_permission(
        &self,
        _user_id: &str,
        _action: &Action,
        _resource: &Resource,
    ) -> Result<bool, AuthError> {
        Ok(true)
    }
}

impl NoAuthProvider {
    /// Create a default superuser session (used when no auth is configured).
    pub fn default_session() -> Session {
        Session {
            user_id: "anonymous".to_string(),
            username: "anonymous".to_string(),
            is_superuser: true,
        }
    }
}
