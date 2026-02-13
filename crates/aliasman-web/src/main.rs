mod error;

use std::net::SocketAddr;
use std::path::PathBuf;

use anyhow::{Context, Result};
use axum::extract::Path;
use axum::http::{header, StatusCode};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use clap::Parser;
use rust_embed::Embed;
use tracing_subscriber::EnvFilter;

use aliasman_core::config::AppConfig;

#[derive(Embed)]
#[folder = "static/"]
struct StaticAssets;

#[derive(Parser)]
#[command(name = "aliasman-web", about = "Aliasman web frontend")]
struct Cli {
    /// Configuration directory
    #[arg(long, default_value_os_t = AppConfig::default_config_dir())]
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

    // Verify config loads (will be used in later steps)
    let _config = AppConfig::load(&cli.config_dir).context("failed to load configuration")?;

    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/static/{*path}", get(static_handler));

    tracing::info!("Starting aliasman-web on {}", cli.bind);
    let listener = tokio::net::TcpListener::bind(cli.bind).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("Server stopped");
    Ok(())
}

async fn health_handler() -> &'static str {
    "ok"
}

async fn static_handler(Path(path): Path<String>) -> impl IntoResponse {
    match StaticAssets::get(&path) {
        Some(content) => {
            let mime = mime_guess::from_path(&path).first_or_octet_stream();
            (
                StatusCode::OK,
                [
                    (header::CONTENT_TYPE, mime.as_ref().to_string()),
                    (header::CACHE_CONTROL, "public, max-age=31536000".to_string()),
                ],
                content.data.into_owned(),
            )
                .into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C signal handler");
    tracing::info!("Shutdown signal received");
}
