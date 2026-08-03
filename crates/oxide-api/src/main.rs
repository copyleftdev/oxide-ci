//! The `oxide-api` server.
//!
//! Wires the adapters to the ports and serves the HTTP/WebSocket API. Every
//! knob is an environment variable, because this runs in a container and the
//! compose file already speaks that language.

use oxide_api::{AppState, ServerConfig, build_app};
use oxide_db::{
    Database, PgAgentRepository, PgApprovalRepository, PgPipelineRepository, PgRunRepository,
};
use oxide_nats::NatsEventBus;
use std::sync::Arc;
use tracing::{info, warn};

/// Read an environment variable, falling back to a default.
fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,oxide=debug".into()),
        )
        .init();

    let database_url = env_or(
        "DATABASE_URL",
        "postgres://oxide:oxide_dev_password@localhost:5432/oxide",
    );
    let nats_url = env_or("NATS_URL", "nats://localhost:4222");
    let config = ServerConfig {
        host: env_or("API_HOST", "0.0.0.0"),
        port: env_or("API_PORT", "8080").parse().unwrap_or(8080),
    };

    info!(%nats_url, "Connecting to services");

    let database = Database::connect(&database_url).await?;
    // Running migrations on start keeps a fresh deployment from needing a
    // separate step; sqlx records what it has applied, so this is a no-op on
    // an already-migrated database.
    database.migrate().await?;
    info!("Database ready");

    let event_bus = NatsEventBus::connect(&nats_url).await?;
    info!("Event bus ready");

    let pool = database.pool().clone();
    let state = Arc::new(AppState::new(
        Arc::new(PgPipelineRepository::new(pool.clone())),
        Arc::new(PgRunRepository::new(pool.clone())),
        Arc::new(PgAgentRepository::new(pool.clone())),
        Arc::new(PgApprovalRepository::new(pool)),
        Arc::new(event_bus),
    ));

    let app = build_app(state);
    let addr = config.addr();
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!(%addr, "API listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("API stopped");
    Ok(())
}

/// Resolve when the process is asked to stop.
///
/// A container runtime sends SIGTERM and then waits; without handling it the
/// process is killed outright and in-flight requests die with it.
async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(e) = tokio::signal::ctrl_c().await {
            warn!(error = %e, "Failed to listen for ctrl-c");
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(e) => warn!(error = %e, "Failed to listen for SIGTERM"),
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => info!("Received ctrl-c, shutting down"),
        _ = terminate => info!("Received SIGTERM, shutting down"),
    }
}
