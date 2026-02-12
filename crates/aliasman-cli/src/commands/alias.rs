use anyhow::{bail, Context, Result};
use clap::Subcommand;

use aliasman_core::email::EmailProvider;
use aliasman_core::model::{generate_random_alias, AliasFilter};
use aliasman_core::storage::StorageProvider;
use aliasman_core::{build_alias, create_alias, delete_alias, list_aliases};

use crate::output::print_alias_table;

#[derive(Subcommand)]
pub enum AliasCommands {
    /// Create an email alias
    Create {
        /// Alias name (omit to use --random)
        #[arg(short, long)]
        alias: Option<String>,

        /// Domain for the alias
        #[arg(short, long)]
        domain: Option<String>,

        /// Target email address(es)
        #[arg(short, long = "email-address")]
        email_address: Vec<String>,

        /// Description of the alias
        #[arg(short = 'D', long)]
        description: Option<String>,

        /// Generate a random alias name
        #[arg(short, long)]
        random: bool,

        /// Length of random alias (max 32)
        #[arg(short = 'l', long, default_value = "16")]
        random_length: usize,
    },

    /// Delete an email alias
    Delete {
        /// Alias name to delete
        #[arg(short, long)]
        alias: String,

        /// Domain of the alias
        #[arg(short, long)]
        domain: Option<String>,
    },

    /// List all aliases
    List,
}

pub async fn handle(
    cmd: &AliasCommands,
    storage: &mut dyn StorageProvider,
    email: &dyn EmailProvider,
    default_domain: Option<&str>,
    default_addresses: Option<&[String]>,
) -> Result<()> {
    match cmd {
        AliasCommands::Create {
            alias,
            domain,
            email_address,
            description,
            random,
            random_length,
        } => {
            let alias_name = if *random {
                generate_random_alias(*random_length)
            } else if let Some(name) = alias {
                name.clone()
            } else {
                bail!("either --alias or --random must be specified");
            };

            let domain = domain
                .as_deref()
                .or(default_domain)
                .context("--domain is required (no default domain configured)")?
                .to_string();

            let addresses = if email_address.is_empty() {
                default_addresses
                    .context("--email-address is required (no default addresses configured)")?
                    .to_vec()
            } else {
                email_address.clone()
            };

            let description = description.clone().unwrap_or_default();

            storage.open(false).await?;
            let alias = build_alias(alias_name, domain, addresses, description);
            let result = create_alias(storage, email, alias).await;
            let close_result = storage.close().await;
            let created = result?;
            close_result?;

            println!("Created alias {}", created);
        }

        AliasCommands::Delete { alias, domain } => {
            let domain = domain
                .as_deref()
                .or(default_domain)
                .context("--domain is required (no default domain configured)")?;

            storage.open(false).await?;
            let result = delete_alias(storage, email, alias, domain).await;
            let close_result = storage.close().await;
            result?;
            close_result?;

            println!("Deleted alias {}@{}", alias, domain);
        }

        AliasCommands::List => {
            storage.open(true).await?;
            let result = list_aliases(storage, &AliasFilter::default()).await;
            let close_result = storage.close().await;
            let aliases = result?;
            close_result?;

            if aliases.is_empty() {
                println!("No aliases found.");
            } else {
                print_alias_table(&aliases);
            }
        }
    }

    Ok(())
}
