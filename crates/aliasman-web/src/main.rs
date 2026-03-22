mod auth;
mod error;
mod routes;
mod state;
pub(crate) mod theme;

use std::net::SocketAddr;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use tracing_subscriber::EnvFilter;

use aliasman_core::config::AppConfig;

use crate::state::AppState;

#[derive(Parser)]
#[command(name = "aliasman-web", about = "Aliasman web frontend")]
struct Cli {
    /// Configuration directory
    #[arg(long, env = "ALIASMAN_CONFIG_DIR", default_value_os_t = AppConfig::default_config_dir())]
    config_dir: PathBuf,

    /// Bind address
    #[arg(long, default_value = "127.0.0.1:3000")]
    bind: SocketAddr,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    let config = AppConfig::load(&cli.config_dir).context("failed to load configuration")?;

    let state = AppState::new(config)
        .await
        .context("failed to initialize application state")?;

    let app = routes::router(state.clone());

    tracing::info!("Starting aliasman-web on {}", cli.bind);
    let listener = tokio::net::TcpListener::bind(cli.bind).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    state.shutdown().await;
    tracing::info!("Server stopped");
    Ok(())
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C signal handler");
    tracing::info!("Shutdown signal received");
}
