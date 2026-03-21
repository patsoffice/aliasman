use anyhow::{bail, Context, Result};
use clap::Subcommand;
use comfy_table::{ContentArrangement, Table};

use aliasman_core::auth::{Action, AuthError, NewUser, Permission, ResourceType, UserStore};
use aliasman_core::config::AppConfig;

#[derive(Subcommand)]
pub enum UserCommands {
    /// Create a new user
    Create {
        /// Username
        username: String,

        /// Password
        #[arg(short, long)]
        password: String,

        /// Grant superuser (full access to everything)
        #[arg(long)]
        superuser: bool,
    },

    /// List all users
    List,

    /// Show a user's details and permissions
    Show {
        /// Username
        username: String,
    },

    /// Delete a user
    Delete {
        /// Username
        username: String,
    },

    /// Reset a user's password
    ResetPassword {
        /// Username
        username: String,

        /// New password
        #[arg(short, long)]
        password: String,
    },

    /// Grant permissions to a user
    Grant {
        /// Username
        username: String,

        /// Grant access to a system (all domains)
        #[arg(long, conflicts_with = "domain")]
        system: Option<String>,

        /// Grant access to a specific domain
        #[arg(long, conflicts_with = "system")]
        domain: Option<String>,

        /// Actions to grant (comma-separated: view,create,delete,suspend,unsuspend)
        /// If omitted, grants all actions
        #[arg(long, value_delimiter = ',')]
        actions: Option<Vec<String>>,
    },

    /// Revoke permissions from a user
    Revoke {
        /// Username
        username: String,

        /// Revoke access to a system
        #[arg(long, conflicts_with = "domain")]
        system: Option<String>,

        /// Revoke access to a specific domain
        #[arg(long, conflicts_with = "system")]
        domain: Option<String>,
    },
}

pub async fn handle(cmd: &UserCommands, config: &AppConfig) -> Result<()> {
    let auth_config = config
        .auth
        .as_ref()
        .context("auth is not configured — add an [auth] section to config.toml")?;

    let mut store =
        aliasman_core::create_user_store(&auth_config.store, auth_config.session_ttl_hours);
    store
        .open()
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
        .context("failed to open user store")?;

    let result = handle_inner(cmd, store.as_ref()).await;

    store.close().await;

    result
}

async fn handle_inner(cmd: &UserCommands, store: &dyn UserStore) -> Result<()> {
    match cmd {
        UserCommands::Create {
            username,
            password,
            superuser,
        } => {
            let new_user = NewUser {
                username: username.clone(),
                password: password.clone(),
                is_superuser: *superuser,
            };

            let user = store
                .create_user(&new_user)
                .await
                .map_err(auth_err_to_anyhow)?;

            println!("Created user '{}' (id: {})", user.username, user.id);
            if user.is_superuser {
                println!("  superuser: yes (full access to everything)");
            }
        }

        UserCommands::List => {
            let users = store.list_users().await.map_err(auth_err_to_anyhow)?;

            if users.is_empty() {
                println!("No users found.");
                return Ok(());
            }

            let mut table = Table::new();
            table
                .set_content_arrangement(ContentArrangement::Dynamic)
                .set_header(vec!["Username", "Superuser", "ID"]);

            for user in &users {
                table.add_row(vec![
                    user.username.clone(),
                    if user.is_superuser {
                        "Yes".to_string()
                    } else {
                        "No".to_string()
                    },
                    user.id.clone(),
                ]);
            }

            println!("{table}");
        }

        UserCommands::Show { username } => {
            let user = store
                .get_user_by_username(username)
                .await
                .map_err(auth_err_to_anyhow)?
                .context(format!("user '{}' not found", username))?;

            println!("User: {}", user.username);
            println!("  ID: {}", user.id);
            println!(
                "  Superuser: {}",
                if user.is_superuser { "yes" } else { "no" }
            );

            let permissions = store
                .get_permissions(&user.id)
                .await
                .map_err(auth_err_to_anyhow)?;

            if permissions.is_empty() && !user.is_superuser {
                println!("  Permissions: none");
            } else if !permissions.is_empty() {
                println!("  Permissions:");
                for perm in &permissions {
                    let resource = perm.resource_id.as_deref().unwrap_or("*");
                    println!(
                        "    {} on {} '{}'",
                        perm.action.as_str(),
                        perm.resource_type.as_str(),
                        resource
                    );
                }
            }
        }

        UserCommands::Delete { username } => {
            store
                .delete_user(username)
                .await
                .map_err(auth_err_to_anyhow)?;
            println!("Deleted user '{}'", username);
        }

        UserCommands::ResetPassword { username, password } => {
            store
                .update_password(username, password)
                .await
                .map_err(auth_err_to_anyhow)?;
            println!("Password updated for user '{}'", username);
        }

        UserCommands::Grant {
            username,
            system,
            domain,
            actions,
        } => {
            let user = store
                .get_user_by_username(username)
                .await
                .map_err(auth_err_to_anyhow)?
                .context(format!("user '{}' not found", username))?;

            let (resource_type, resource_id) = resolve_resource(system, domain)?;

            let action_list = parse_actions(actions)?;

            let permissions: Vec<Permission> = action_list
                .iter()
                .map(|action| Permission {
                    id: String::new(),
                    user_id: user.id.clone(),
                    action: action.clone(),
                    resource_type: resource_type.clone(),
                    resource_id: Some(resource_id.clone()),
                })
                .collect();

            store
                .set_permissions(&user.id, &permissions)
                .await
                .map_err(auth_err_to_anyhow)?;

            let action_names: Vec<&str> = action_list.iter().map(|a| a.as_str()).collect();
            println!(
                "Granted [{}] on {} '{}' to user '{}'",
                action_names.join(", "),
                resource_type.as_str(),
                resource_id,
                username
            );
        }

        UserCommands::Revoke {
            username,
            system,
            domain,
        } => {
            let user = store
                .get_user_by_username(username)
                .await
                .map_err(auth_err_to_anyhow)?
                .context(format!("user '{}' not found", username))?;

            let (resource_type, resource_id) = resolve_resource(system, domain)?;

            store
                .clear_permissions(&user.id, &resource_type, &resource_id)
                .await
                .map_err(auth_err_to_anyhow)?;

            println!(
                "Revoked all permissions on {} '{}' from user '{}'",
                resource_type.as_str(),
                resource_id,
                username
            );
        }
    }

    Ok(())
}

fn resolve_resource(
    system: &Option<String>,
    domain: &Option<String>,
) -> Result<(ResourceType, String)> {
    match (system, domain) {
        (Some(s), None) => Ok((ResourceType::System, s.clone())),
        (None, Some(d)) => Ok((ResourceType::Domain, d.clone())),
        _ => bail!("exactly one of --system or --domain must be specified"),
    }
}

fn parse_actions(actions: &Option<Vec<String>>) -> Result<Vec<Action>> {
    match actions {
        None => Ok(Action::all().to_vec()),
        Some(names) => names
            .iter()
            .map(|name| Action::parse(name).map_err(|e| anyhow::anyhow!("{}", e)))
            .collect(),
    }
}

fn auth_err_to_anyhow(e: AuthError) -> anyhow::Error {
    anyhow::anyhow!("{}", e)
}
