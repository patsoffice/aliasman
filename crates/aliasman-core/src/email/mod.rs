pub mod rackspace;

use async_trait::async_trait;

use crate::error::Result;
use crate::model::Alias;

#[async_trait]
pub trait EmailProvider: Send + Sync {
    /// Create an alias on the email provider, pointing to the given addresses.
    async fn alias_create(&self, alias: &str, domain: &str, addresses: &[String]) -> Result<()>;

    /// Delete an alias from the email provider.
    async fn alias_delete(&self, alias: &str, domain: &str) -> Result<()>;

    /// List all aliases for a domain from the email provider.
    async fn alias_list(&self, domain: &str) -> Result<Vec<Alias>>;
}
