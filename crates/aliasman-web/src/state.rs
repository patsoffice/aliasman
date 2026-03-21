use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use aliasman_core::auth::UserStore;
use aliasman_core::config::AppConfig;
use aliasman_core::error::Result as CoreResult;
use aliasman_core::model::{Alias, AliasFilter};
use aliasman_core::storage::StorageProvider;
use aliasman_core::{create_email_provider, create_storage_provider, create_user_store};

pub type SharedState = Arc<AppState>;

pub struct AppState {
    config: AppConfig,
    systems: RwLock<HashMap<String, Box<dyn StorageProvider>>>,
    active_system: RwLock<String>,
    user_store: Option<Box<dyn UserStore>>,
}

impl AppState {
    pub async fn new(config: AppConfig) -> CoreResult<SharedState> {
        let default_system = config.default_system.clone();
        let system_config = config.system(Some(&default_system))?;

        let mut storage = create_storage_provider(&system_config.storage);
        storage.open(false).await?;

        let mut systems = HashMap::new();
        systems.insert(default_system.clone(), storage);

        // Open user store if auth is configured
        let user_store = if let Some(ref auth_config) = config.auth {
            let mut store = create_user_store(&auth_config.store, auth_config.session_ttl_hours);
            store.open().await.map_err(|e| {
                aliasman_core::error::Error::Config(format!("failed to open user store: {}", e))
            })?;

            let users = store.list_users().await.map_err(|e| {
                aliasman_core::error::Error::Config(format!("failed to list users: {}", e))
            })?;
            if users.is_empty() {
                tracing::warn!(
                    "Auth enabled but no users found. Use `aliasman user create --superuser` to create an admin."
                );
            }

            Some(store)
        } else {
            None
        };

        Ok(Arc::new(Self {
            config,
            systems: RwLock::new(systems),
            active_system: RwLock::new(default_system),
            user_store,
        }))
    }

    /// Returns a reference to the user store, if auth is configured.
    pub fn user_store(&self) -> Option<&dyn UserStore> {
        self.user_store.as_deref()
    }

    /// Returns true if auth is configured.
    pub fn auth_enabled(&self) -> bool {
        self.user_store.is_some()
    }

    pub async fn active_system_name(&self) -> String {
        self.active_system.read().await.clone()
    }

    pub fn system_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.config.systems.keys().cloned().collect();
        names.sort();
        names
    }

    /// Get the active system's default domain from config.
    pub async fn active_default_domain(&self) -> Option<String> {
        let active = self.active_system.read().await.clone();
        self.config
            .system(Some(&active))
            .ok()
            .and_then(|s| s.domain.clone())
    }

    /// Get the active system's default email addresses from config.
    pub async fn active_default_addresses(&self) -> Option<Vec<String>> {
        let active = self.active_system.read().await.clone();
        self.config
            .system(Some(&active))
            .ok()
            .and_then(|s| s.email_addresses.clone())
    }

    /// Create an alias on the active system (dual-write: email provider first, then storage).
    pub async fn create_alias(&self, alias: Alias) -> CoreResult<Alias> {
        let active = self.active_system.read().await.clone();
        let system_config = self.config.system(Some(&active))?;
        let email = create_email_provider(&system_config.email)?;

        let systems = self.systems.read().await;
        let storage = systems.get(&active).ok_or_else(|| {
            aliasman_core::error::Error::Config(format!("active system '{}' not found", active))
        })?;
        aliasman_core::create_alias(storage.as_ref(), email.as_ref(), alias).await
    }

    /// Edit an alias on the active system.
    pub async fn edit_alias(
        &self,
        alias: &str,
        domain: &str,
        new_addresses: Option<Vec<String>>,
        new_description: Option<String>,
    ) -> CoreResult<Alias> {
        let active = self.active_system.read().await.clone();
        let system_config = self.config.system(Some(&active))?;
        let email = create_email_provider(&system_config.email)?;

        let systems = self.systems.read().await;
        let storage = systems.get(&active).ok_or_else(|| {
            aliasman_core::error::Error::Config(format!("active system '{}' not found", active))
        })?;
        aliasman_core::edit_alias(
            storage.as_ref(),
            email.as_ref(),
            alias,
            domain,
            new_addresses,
            new_description,
        )
        .await
    }

    /// Delete an alias from the active system (dual-write: email provider first, then storage).
    pub async fn delete_alias(&self, alias: &str, domain: &str) -> CoreResult<()> {
        let active = self.active_system.read().await.clone();
        let system_config = self.config.system(Some(&active))?;
        let email = create_email_provider(&system_config.email)?;

        let systems = self.systems.read().await;
        let storage = systems.get(&active).ok_or_else(|| {
            aliasman_core::error::Error::Config(format!("active system '{}' not found", active))
        })?;
        aliasman_core::delete_alias(storage.as_ref(), email.as_ref(), alias, domain).await
    }

    /// Suspend an alias on the active system.
    pub async fn suspend_alias(&self, alias: &str, domain: &str) -> CoreResult<()> {
        let active = self.active_system.read().await.clone();
        let system_config = self.config.system(Some(&active))?;
        let email = create_email_provider(&system_config.email)?;

        let systems = self.systems.read().await;
        let storage = systems.get(&active).ok_or_else(|| {
            aliasman_core::error::Error::Config(format!("active system '{}' not found", active))
        })?;
        aliasman_core::suspend_alias(storage.as_ref(), email.as_ref(), alias, domain).await
    }

    /// Unsuspend an alias on the active system.
    pub async fn unsuspend_alias(&self, alias: &str, domain: &str) -> CoreResult<()> {
        let active = self.active_system.read().await.clone();
        let system_config = self.config.system(Some(&active))?;
        let email = create_email_provider(&system_config.email)?;

        let systems = self.systems.read().await;
        let storage = systems.get(&active).ok_or_else(|| {
            aliasman_core::error::Error::Config(format!("active system '{}' not found", active))
        })?;
        aliasman_core::unsuspend_alias(storage.as_ref(), email.as_ref(), alias, domain).await
    }

    pub async fn list_aliases(&self, filter: &AliasFilter) -> CoreResult<Vec<Alias>> {
        let active = self.active_system.read().await.clone();
        let systems = self.systems.read().await;
        let storage = systems.get(&active).ok_or_else(|| {
            aliasman_core::error::Error::Config(format!("active system '{}' not found", active))
        })?;
        aliasman_core::list_aliases(storage.as_ref(), filter).await
    }

    pub async fn switch_system(&self, name: &str) -> CoreResult<()> {
        let system_config = self.config.system(Some(name))?;

        let mut systems = self.systems.write().await;
        if !systems.contains_key(name) {
            let mut storage = create_storage_provider(&system_config.storage);
            storage.open(false).await?;
            systems.insert(name.to_string(), storage);
        }
        drop(systems);

        *self.active_system.write().await = name.to_string();
        Ok(())
    }

    pub async fn refresh_active_system(&self) -> CoreResult<()> {
        let active = self.active_system.read().await.clone();
        let mut systems = self.systems.write().await;
        if let Some(storage) = systems.get_mut(&active) {
            storage.refresh().await?;
        }
        Ok(())
    }

    pub async fn shutdown(&self) {
        let mut systems = self.systems.write().await;
        for (name, storage) in systems.iter_mut() {
            if let Err(e) = storage.close().await {
                tracing::error!("Failed to close storage for system '{}': {}", name, e);
            }
        }
    }
}
