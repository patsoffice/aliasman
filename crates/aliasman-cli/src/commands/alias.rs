use anyhow::{bail, Context, Result};
use clap::Subcommand;
use regex::Regex;

use aliasman_core::email::EmailProvider;
use aliasman_core::model::{generate_random_alias, AliasFilter};
use aliasman_core::storage::StorageProvider;
use aliasman_core::{
    build_alias, create_alias, delete_alias, list_aliases, suspend_alias, unsuspend_alias,
};

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

    /// Suspend an email alias (stops email routing)
    Suspend {
        /// Alias name to suspend
        #[arg(short, long)]
        alias: String,

        /// Domain of the alias
        #[arg(short, long)]
        domain: Option<String>,
    },

    /// Unsuspend an email alias (restarts email routing)
    Unsuspend {
        /// Alias name to unsuspend
        #[arg(short, long)]
        alias: String,

        /// Domain of the alias
        #[arg(short, long)]
        domain: Option<String>,
    },

    /// Search for aliases via regular expressions
    Search {
        /// Regular expression for matching aliases (searches alias, domain, email addresses, and description)
        #[arg(short, long)]
        search_regexp: Option<String>,

        /// Exclude suspended aliases from results
        #[arg(short = 'e', long)]
        exclude_suspended: bool,

        /// Exclude enabled (non-suspended) aliases from results
        #[arg(short = 'E', long)]
        exclude_enabled: bool,
    },
}

pub async fn handle(
    cmd: &AliasCommands,
    storage: &mut dyn StorageProvider,
    email: &dyn EmailProvider,
    default_domain: Option<&str>,
    default_addresses: Option<&[String]>,
    dry_run: bool,
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

            let alias = build_alias(alias_name, domain, addresses, description);

            if dry_run {
                storage.open(true).await?;
                if storage.get(&alias.alias, &alias.domain).await?.is_some() {
                    bail!("alias '{}' already exists", alias.full_alias());
                }
                storage.close().await?;
                println!("Would create alias {}", alias);
            } else {
                storage.open(false).await?;
                let result = create_alias(storage, email, alias).await;
                let close_result = storage.close().await;
                let created = result?;
                close_result?;
                println!("Created alias {}", created);
            }
        }

        AliasCommands::Delete { alias, domain } => {
            let domain = domain
                .as_deref()
                .or(default_domain)
                .context("--domain is required (no default domain configured)")?;

            if dry_run {
                storage.open(true).await?;
                if storage.get(alias, domain).await?.is_none() {
                    bail!("alias '{}@{}' not found", alias, domain);
                }
                storage.close().await?;
                println!("Would delete alias {}@{}", alias, domain);
            } else {
                storage.open(false).await?;
                let result = delete_alias(storage, email, alias, domain).await;
                let close_result = storage.close().await;
                result?;
                close_result?;
                println!("Deleted alias {}@{}", alias, domain);
            }
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

        AliasCommands::Suspend { alias, domain } => {
            let domain = domain
                .as_deref()
                .or(default_domain)
                .context("--domain is required (no default domain configured)")?;

            if dry_run {
                storage.open(true).await?;
                match storage.get(alias, domain).await? {
                    None => bail!("alias '{}@{}' not found", alias, domain),
                    Some(a) if a.suspended => {
                        bail!("alias '{}@{}' is already suspended", alias, domain)
                    }
                    _ => {}
                }
                storage.close().await?;
                println!("Would suspend alias {}@{}", alias, domain);
            } else {
                storage.open(false).await?;
                let result = suspend_alias(storage, email, alias, domain).await;
                let close_result = storage.close().await;
                result?;
                close_result?;
                println!("Suspended alias {}@{}", alias, domain);
            }
        }

        AliasCommands::Unsuspend { alias, domain } => {
            let domain = domain
                .as_deref()
                .or(default_domain)
                .context("--domain is required (no default domain configured)")?;

            if dry_run {
                storage.open(true).await?;
                match storage.get(alias, domain).await? {
                    None => bail!("alias '{}@{}' not found", alias, domain),
                    Some(a) if !a.suspended => {
                        bail!("alias '{}@{}' is not suspended", alias, domain)
                    }
                    _ => {}
                }
                storage.close().await?;
                println!("Would unsuspend alias {}@{}", alias, domain);
            } else {
                storage.open(false).await?;
                let result = unsuspend_alias(storage, email, alias, domain).await;
                let close_result = storage.close().await;
                result?;
                close_result?;
                println!("Unsuspended alias {}@{}", alias, domain);
            }
        }

        AliasCommands::Search {
            search_regexp,
            exclude_suspended,
            exclude_enabled,
        } => {
            let re = search_regexp
                .as_deref()
                .map(Regex::new)
                .transpose()
                .context("invalid --search-regexp regex")?;
            let filter = AliasFilter {
                alias: re.clone(),
                domain: re.clone(),
                email_address: re.clone(),
                description: re,
                exclude_suspended: *exclude_suspended,
                exclude_enabled: *exclude_enabled,
            };

            storage.open(true).await?;
            let result = list_aliases(storage, &filter).await;
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
