use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// Top-level application configuration.
///
/// Supports multiple named "systems", each pairing a storage provider with an
/// email provider. The `default_system` field selects which system to use when
/// none is specified on the command line.
///
/// Example config.toml:
/// ```toml
/// default_system = "home"
///
/// [systems.home]
/// domain = "example.com"
/// email_addresses = ["user@example.com"]
///
/// [systems.home.storage]
/// type = "sqlite"
/// db_path = "~/.config/aliasman/home.db"
///
/// [systems.home.email]
/// type = "rackspace"
/// user_key = "home-key"
/// secret_key = "home-secret"
///
/// [systems.work]
/// domain = "work.com"
/// email_addresses = ["me@work.com"]
///
/// [systems.work.storage]
/// type = "sqlite"
/// db_path = "~/.config/aliasman/work.db"
///
/// [systems.work.email]
/// type = "rackspace"
/// user_key = "work-key"
/// secret_key = "work-secret"
/// ```
#[derive(Debug, Deserialize, Serialize)]
pub struct AppConfig {
    pub default_system: String,
    pub systems: HashMap<String, SystemConfig>,
    #[serde(default)]
    pub auth: Option<AuthConfig>,
}

/// Authentication configuration.
#[derive(Debug, Deserialize, Serialize)]
pub struct AuthConfig {
    /// User store backend configuration.
    pub store: UserStoreConfig,
    /// Session time-to-live in hours. Defaults to 24.
    #[serde(default = "default_session_ttl_hours")]
    pub session_ttl_hours: u64,
}

/// User store backend configuration.
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum UserStoreConfig {
    #[serde(rename = "sqlite")]
    Sqlite { db_path: String },

    #[serde(rename = "postgres")]
    Postgres { url: String },
}

fn default_session_ttl_hours() -> u64 {
    24
}

/// A named system combining storage, email, and default values.
#[derive(Debug, Deserialize, Serialize)]
pub struct SystemConfig {
    pub storage: StorageConfig,
    pub email: EmailConfig,
    #[serde(default)]
    pub domain: Option<String>,
    #[serde(default)]
    pub email_addresses: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum StorageConfig {
    #[serde(rename = "sqlite")]
    Sqlite { db_path: String },

    #[serde(rename = "postgres")]
    Postgres {
        /// PostgreSQL connection URL (e.g. postgres://user:pass@host/db)
        url: String,
    },

    #[serde(rename = "s3")]
    S3 {
        bucket: String,
        #[serde(default)]
        region: Option<String>,
        #[serde(default)]
        endpoint: Option<String>,
        #[serde(default)]
        access_key_id: Option<String>,
        #[serde(default)]
        secret_access_key: Option<String>,
    },
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum EmailConfig {
    #[serde(rename = "rackspace")]
    Rackspace {
        user_key: String,
        secret_key: String,
    },
}

impl AppConfig {
    /// Load configuration from the given directory.
    /// Looks for `config.toml` in the directory and merges with environment
    /// variables prefixed with `ALIASMAN_`.
    pub fn load(config_dir: &Path) -> Result<Self> {
        let config_file = config_dir.join("config.toml");

        let settings = config::Config::builder()
            .add_source(config::File::from(config_file).required(true))
            .add_source(
                config::Environment::with_prefix("ALIASMAN")
                    .separator("_")
                    .try_parsing(true),
            )
            .build()
            .map_err(|e| Error::Config(e.to_string()))?;

        settings
            .try_deserialize()
            .map_err(|e| Error::Config(e.to_string()))
    }

    /// Look up a system by name, falling back to `default_system`.
    pub fn system(&self, name: Option<&str>) -> Result<&SystemConfig> {
        let key = name.unwrap_or(&self.default_system);
        self.systems
            .get(key)
            .ok_or_else(|| Error::Config(format!("system '{}' not found in config", key)))
    }

    /// Returns the default config directory (~/.config/aliasman).
    pub fn default_config_dir() -> PathBuf {
        dirs_config_dir().join("aliasman")
    }

    /// Expand a path string, resolving `~` to the home directory.
    pub fn expand_path(path: &str) -> PathBuf {
        let expanded = shellexpand::tilde(path);
        PathBuf::from(expanded.as_ref())
    }
}

fn dirs_config_dir() -> PathBuf {
    dirs::config_dir().unwrap_or_else(|| PathBuf::from("~/.config"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_deserialize_config() {
        let toml = r#"
default_system = "home"

[systems.home]
domain = "example.com"
email_addresses = ["user@example.com"]

[systems.home.storage]
type = "sqlite"
db_path = "~/.config/aliasman/home.db"

[systems.home.email]
type = "rackspace"
user_key = "test-key"
secret_key = "test-secret"
"#;
        let config: AppConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.default_system, "home");

        let system = config.system(None).unwrap();
        assert_eq!(system.domain.as_deref(), Some("example.com"));

        match &system.storage {
            StorageConfig::Sqlite { db_path } => {
                assert_eq!(db_path, "~/.config/aliasman/home.db");
            }
            _ => panic!("expected SQLite storage"),
        }

        match &system.email {
            EmailConfig::Rackspace {
                user_key,
                secret_key,
            } => {
                assert_eq!(user_key, "test-key");
                assert_eq!(secret_key, "test-secret");
            }
        }
    }

    #[test]
    fn test_multiple_systems() {
        let toml = r#"
default_system = "home"

[systems.home]
domain = "home.com"

[systems.home.storage]
type = "sqlite"
db_path = "/tmp/home.db"

[systems.home.email]
type = "rackspace"
user_key = "home-key"
secret_key = "home-secret"

[systems.work]
domain = "work.com"

[systems.work.storage]
type = "sqlite"
db_path = "/tmp/work.db"

[systems.work.email]
type = "rackspace"
user_key = "work-key"
secret_key = "work-secret"
"#;
        let config: AppConfig = toml::from_str(toml).unwrap();

        let home = config.system(Some("home")).unwrap();
        assert_eq!(home.domain.as_deref(), Some("home.com"));

        let work = config.system(Some("work")).unwrap();
        assert_eq!(work.domain.as_deref(), Some("work.com"));

        // default falls back to "home"
        let default = config.system(None).unwrap();
        assert_eq!(default.domain.as_deref(), Some("home.com"));
    }

    #[test]
    fn test_system_not_found() {
        let toml = r#"
default_system = "home"

[systems.home]

[systems.home.storage]
type = "sqlite"
db_path = "/tmp/test.db"

[systems.home.email]
type = "rackspace"
user_key = "k"
secret_key = "s"
"#;
        let config: AppConfig = toml::from_str(toml).unwrap();
        let result = config.system(Some("nonexistent"));
        assert!(result.is_err());
    }

    #[test]
    fn test_load_config_from_file() {
        let dir = tempfile::tempdir().unwrap();
        let toml = r#"
default_system = "main"

[systems.main]

[systems.main.storage]
type = "sqlite"
db_path = "/tmp/test.db"

[systems.main.email]
type = "rackspace"
user_key = "key"
secret_key = "secret"
"#;
        fs::write(dir.path().join("config.toml"), toml).unwrap();

        let config = AppConfig::load(dir.path()).unwrap();
        let system = config.system(None).unwrap();
        match &system.storage {
            StorageConfig::Sqlite { db_path } => {
                assert_eq!(db_path, "/tmp/test.db");
            }
            _ => panic!("expected SQLite storage"),
        }
    }

    #[test]
    fn test_expand_path_tilde() {
        let expanded = AppConfig::expand_path("~/test");
        assert!(!expanded.to_string_lossy().contains('~'));
        assert!(expanded.to_string_lossy().ends_with("/test"));
    }

    #[test]
    fn test_deserialize_s3_config() {
        let toml = r#"
default_system = "home-s3"

[systems.home-s3]
domain = "example.com"
email_addresses = ["user@example.com"]

[systems.home-s3.storage]
type = "s3"
bucket = "my-aliasman-bucket"
region = "us-east-1"

[systems.home-s3.email]
type = "rackspace"
user_key = "test-key"
secret_key = "test-secret"
"#;
        let config: AppConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.default_system, "home-s3");

        let system = config.system(None).unwrap();
        match &system.storage {
            StorageConfig::S3 {
                bucket,
                region,
                endpoint,
                access_key_id,
                secret_access_key,
            } => {
                assert_eq!(bucket, "my-aliasman-bucket");
                assert_eq!(region.as_deref(), Some("us-east-1"));
                assert!(endpoint.is_none());
                assert!(access_key_id.is_none());
                assert!(secret_access_key.is_none());
            }
            _ => panic!("expected S3 storage"),
        }
    }

    #[test]
    fn test_deserialize_s3_config_with_credentials() {
        let toml = r#"
default_system = "home-s3"

[systems.home-s3.storage]
type = "s3"
bucket = "my-bucket"
region = "eu-west-1"
endpoint = "http://localhost:9000"
access_key_id = "AKIAIOSFODNN7EXAMPLE"
secret_access_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"

[systems.home-s3.email]
type = "rackspace"
user_key = "key"
secret_key = "secret"
"#;
        let config: AppConfig = toml::from_str(toml).unwrap();
        let system = config.system(None).unwrap();
        match &system.storage {
            StorageConfig::S3 {
                bucket,
                region,
                endpoint,
                access_key_id,
                secret_access_key,
            } => {
                assert_eq!(bucket, "my-bucket");
                assert_eq!(region.as_deref(), Some("eu-west-1"));
                assert_eq!(endpoint.as_deref(), Some("http://localhost:9000"));
                assert_eq!(access_key_id.as_deref(), Some("AKIAIOSFODNN7EXAMPLE"));
                assert_eq!(
                    secret_access_key.as_deref(),
                    Some("wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY")
                );
            }
            _ => panic!("expected S3 storage"),
        }
    }
}
