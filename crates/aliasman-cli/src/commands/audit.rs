use anyhow::{Context, Result};

use aliasman_core::email::EmailProvider;
use aliasman_core::storage::StorageProvider;
use aliasman_core::{audit_aliases, AuditDiff};

pub async fn handle(
    storage: &mut dyn StorageProvider,
    email: &dyn EmailProvider,
    domain: &str,
) -> Result<()> {
    storage.open(true).await?;
    let result = audit_aliases(storage, email, domain)
        .await
        .context("audit failed")?;
    storage.close().await?;

    println!(
        "Auditing domain: {} ({} in storage, {} on email provider)",
        domain, result.storage_count, result.email_count
    );

    if result.diffs.is_empty() {
        println!("No differences found.");
        return Ok(());
    }

    println!("\n{} difference(s) found:\n", result.diffs.len());

    for diff in &result.diffs {
        match diff {
            AuditDiff::StorageOnly {
                alias, domain, ..
            } => {
                println!("  MISSING FROM EMAIL  {}@{}", alias, domain);
                println!("    Active in storage but not found on email provider");
            }
            AuditDiff::EmailOnly {
                alias,
                domain,
                email_addresses,
            } => {
                println!("  MISSING FROM STORAGE  {}@{}", alias, domain);
                println!(
                    "    On email provider (-> {}) but not in storage",
                    email_addresses.join(", ")
                );
            }
            AuditDiff::AddressMismatch {
                alias,
                domain,
                storage_addresses,
                email_addresses,
            } => {
                println!("  ADDRESS MISMATCH  {}@{}", alias, domain);
                println!("    Storage: {}", storage_addresses.join(", "));
                println!("    Email:   {}", email_addresses.join(", "));
            }
        }
        println!();
    }

    Ok(())
}
