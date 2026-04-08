use anyhow::{Context, Result};

use aliasman_core::email::EmailProvider;
use aliasman_core::storage::StorageProvider;
use aliasman_core::{sync_aliases, sync_from_email};

pub async fn handle_sync(
    storage: &mut dyn StorageProvider,
    email: &dyn EmailProvider,
    domain: &str,
    dry_run: bool,
) -> Result<()> {
    storage.open(false).await?;
    let result = sync_aliases(storage, email, domain, dry_run)
        .await
        .context("sync failed")?;
    storage.close().await?;

    if dry_run {
        println!("Dry run — no changes made.");
    }

    println!(
        "Sync to email complete: {} created, {} deleted, {} updated",
        result.created, result.deleted, result.updated
    );

    Ok(())
}

pub async fn handle_sync_from_email(
    storage: &mut dyn StorageProvider,
    email: &dyn EmailProvider,
    domain: &str,
    dry_run: bool,
) -> Result<()> {
    storage.open(false).await?;
    let result = sync_from_email(storage, email, domain, dry_run)
        .await
        .context("sync-from-email failed")?;
    storage.close().await?;

    if dry_run {
        println!("Dry run — no changes made.");
    }

    println!(
        "Sync from email complete: {} created, {} deleted, {} updated",
        result.created, result.deleted, result.updated
    );

    Ok(())
}
