use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("alias '{alias}@{domain}' not found")]
    AliasNotFound { alias: String, domain: String },

    #[error("alias '{alias}@{domain}' already exists")]
    AliasAlreadyExists { alias: String, domain: String },

    #[error("storage error: {0}")]
    Storage(#[source] Box<dyn std::error::Error + Send + Sync>),

    #[error("email provider error: {0}")]
    Email(#[source] Box<dyn std::error::Error + Send + Sync>),

    #[error("configuration error: {0}")]
    Config(String),

    #[error("invalid input: {0}")]
    InvalidInput(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl From<sqlx::Error> for Error {
    fn from(e: sqlx::Error) -> Self {
        Error::Storage(Box::new(e))
    }
}

impl From<rackspace_email::ApiError> for Error {
    fn from(e: rackspace_email::ApiError) -> Self {
        Error::Email(Box::new(e))
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Storage(Box::new(e))
    }
}
