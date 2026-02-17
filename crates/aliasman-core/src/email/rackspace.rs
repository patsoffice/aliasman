use async_trait::async_trait;
use chrono::Utc;
use tracing::{info, instrument};

use crate::email::EmailProvider;
use crate::error::{Error, Result};
use crate::model::Alias;

/// Trait to abstract the Rackspace Client for testing purposes.
#[async_trait]
pub trait RackspaceClientImpl: Send + Sync {
    async fn create_alias(
        &self,
        domain: &str,
        alias: &rackspace_email::Alias,
    ) -> std::result::Result<(), rackspace_email::ApiError>;

    async fn delete_alias(
        &self,
        domain: &str,
        alias: &str,
    ) -> std::result::Result<(), rackspace_email::ApiError>;

    async fn list_aliases(
        &self,
        domain: &str,
        page_size: Option<usize>,
    ) -> std::result::Result<Vec<rackspace_email::Alias>, rackspace_email::ApiError>;
}

#[async_trait]
impl RackspaceClientImpl for rackspace_email::RackspaceClient {
    async fn create_alias(
        &self,
        domain: &str,
        alias: &rackspace_email::Alias,
    ) -> std::result::Result<(), rackspace_email::ApiError> {
        self.create_rackspace_alias(domain, alias).await
    }

    async fn delete_alias(
        &self,
        domain: &str,
        alias: &str,
    ) -> std::result::Result<(), rackspace_email::ApiError> {
        self.delete_rackspace_alias(domain, alias).await
    }

    async fn list_aliases(
        &self,
        domain: &str,
        page_size: Option<usize>,
    ) -> std::result::Result<Vec<rackspace_email::Alias>, rackspace_email::ApiError> {
        self.list_rackspace_aliases(domain, page_size).await
    }
}

/// Implementation of `EmailProvider` for Rackspace Email.
pub struct RackspaceEmailProvider {
    client: Box<dyn RackspaceClientImpl>,
}

impl RackspaceEmailProvider {
    /// Creates a new `RackspaceEmailProvider` with the given credentials.
    pub fn new(user_key: &str, secret_key: &str) -> Result<Self> {
        let client = rackspace_email::RackspaceClient::new(user_key, secret_key, None, None)
            .map_err(|e| Error::Email(Box::new(e)))?;
        Ok(Self {
            client: Box::new(client),
        })
    }
}

#[async_trait]
impl EmailProvider for RackspaceEmailProvider {
    #[instrument(skip(self))]
    async fn alias_create(&self, alias: &str, domain: &str, addresses: &[String]) -> Result<()> {
        info!("Creating alias {}@{} -> {:?}", alias, domain, addresses);
        let rs_alias = rackspace_email::Alias {
            alias: alias.to_string(),
            email_list: addresses.to_vec(),
        };

        self.client
            .create_alias(domain, &rs_alias)
            .await
            .map_err(|e| Error::Email(Box::new(e)))?;

        Ok(())
    }

    #[instrument(skip(self))]
    async fn alias_delete(&self, alias: &str, domain: &str) -> Result<()> {
        info!("Deleting alias {}@{}", alias, domain);
        self.client
            .delete_alias(domain, alias)
            .await
            .map_err(|e| Error::Email(Box::new(e)))?;

        Ok(())
    }

    #[instrument(skip(self))]
    async fn alias_list(&self, domain: &str) -> Result<Vec<Alias>> {
        info!("Listing aliases for domain {}", domain);
        let rs_aliases = self
            .client
            .list_aliases(domain, None)
            .await
            .map_err(|e| Error::Email(Box::new(e)))?;

        let now = Utc::now();
        let aliases = rs_aliases
            .into_iter()
            .map(|ra| Alias {
                alias: ra.alias,
                domain: domain.to_string(),
                email_addresses: ra.email_list,
                description: String::new(),
                suspended: false,
                created_at: now,  // Rackspace API does not return timestamps
                modified_at: now, // Rackspace API does not return timestamps
                suspended_at: None,
            })
            .collect();

        Ok(aliases)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    type CreateCallLog = Arc<Mutex<Vec<(String, String, Vec<String>)>>>;

    struct MockRackspaceClient {
        // Stores (domain, alias, email_list) for verification
        create_calls: CreateCallLog,
    }

    #[async_trait]
    impl RackspaceClientImpl for MockRackspaceClient {
        async fn create_alias(
            &self,
            domain: &str,
            alias: &rackspace_email::Alias,
        ) -> std::result::Result<(), rackspace_email::ApiError> {
            self.create_calls.lock().unwrap().push((
                domain.to_string(),
                alias.alias.clone(),
                alias.email_list.clone(),
            ));
            Ok(())
        }

        async fn delete_alias(
            &self,
            _domain: &str,
            _alias: &str,
        ) -> std::result::Result<(), rackspace_email::ApiError> {
            unimplemented!("Not needed for this test")
        }

        async fn list_aliases(
            &self,
            _domain: &str,
            _page: Option<usize>,
        ) -> std::result::Result<Vec<rackspace_email::Alias>, rackspace_email::ApiError> {
            unimplemented!("Not needed for this test")
        }
    }

    #[tokio::test]
    async fn test_alias_create() {
        let create_calls = Arc::new(Mutex::new(Vec::new()));
        let mock = MockRackspaceClient {
            create_calls: create_calls.clone(),
        };
        let provider = RackspaceEmailProvider {
            client: Box::new(mock),
        };

        let addresses = vec!["target@example.com".to_string()];
        provider
            .alias_create("test", "example.com", &addresses)
            .await
            .unwrap();

        let calls = create_calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(
            calls[0],
            ("example.com".to_string(), "test".to_string(), addresses)
        );
    }
}
