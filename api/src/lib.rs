mod user;
mod telemetry;
mod proxy;

use anyhow::Result;
use axum::Router;
use std::env;
use std::sync::OnceLock;
use tokio::signal;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing::{error, info};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt, fmt::writer::MakeWriterExt};
use tracing_appender::{rolling::{RollingFileAppender, Rotation}, non_blocking};

use crate::proxy::{ProxyConfig, ProxyService};

static AMP_API_KEY: OnceLock<String> = OnceLock::new();

pub fn get_amp_api_key() -> &'static str {
    AMP_API_KEY.get().expect("AMP_API_KEY not initialized")
}

#[tokio::main]
async fn start() -> Result<()> {
    // Load .env if present
    let _ = dotenvy::dotenv();

    // Initialize tracing with file logging
    let log_level = env::var("RUST_LOG").unwrap_or_else(|_| "info".into());
    
    // Create logs directory if it doesn't exist
    std::fs::create_dir_all("logs").unwrap_or_else(|e| {
        eprintln!("Warning: Could not create logs directory: {}", e);
    });
    
    // Create file appender (daily rotation)
    let file_appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix("amp-server")
        .filename_suffix("log")
        .build("logs")?;
    let (non_blocking_file, _guard) = non_blocking(file_appender);
    
    // Initialize subscriber with both console and file output
    tracing_subscriber::registry()
        .with(EnvFilter::new(log_level.clone()))
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stdout.and(non_blocking_file))
                .with_ansi(false) // Disable ANSI colors for file output
        )
        .try_init()?;
        
    info!("Logging initialized with level: {}", log_level);
    info!("Logs will be written to: logs/amp-server.log");
    
    // Keep the guard alive for the duration of the program
    let _log_guard = _guard;

    // Load environment variables (with hardcoded fallbacks)
    let host = env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let amp_api_key = env::var("AMP_API_KEY").expect("AMP_API_KEY environment variable is required");
    AMP_API_KEY.set(amp_api_key).expect("AMP_API_KEY already initialized");
    let server_url = format!("{host}:{port}");
    
    // Load proxy configuration
    let proxy_config = ProxyConfig::load_from_file("proxy_config.yaml")
        .unwrap_or_else(|e| {
            info!("Using default proxy configuration ({})", e);
            ProxyConfig::default()
        });
    
    // Create proxy service
    let proxy_service = ProxyService::new(proxy_config);
    
    // Initialize router
    let app = Router::new()
        .merge(user::router())
        .merge(telemetry::router())
        .merge(proxy_service.create_router())
        .layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()));

    // Start server
    let listener = tokio::net::TcpListener::bind(&server_url).await?;
    info!("Listening on {}", server_url);
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("Received termination signal shutting down");
}

pub fn main() {
    let result = start();
    if let Err(err) = result {
        error!("Error: {err}");
    }
}
