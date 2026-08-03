//! The `oxide-agent` worker.
//!
//! Registers with the control plane, then takes jobs off the event bus and
//! runs them. Configuration comes from a YAML file when one is given and from
//! the environment otherwise, so the same binary works on a developer's
//! machine and in a container without a config file being mandatory.

use oxide_agent::{AgentConfig, BuildAgent};
use oxide_db::{Database, PgAgentRepository};
use oxide_nats::NatsEventBus;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{info, warn};

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

/// Build the agent's configuration.
///
/// A config file wins if `AGENT_CONFIG` points at one; otherwise the
/// environment fills in over the defaults.
fn load_config() -> Result<AgentConfig, Box<dyn std::error::Error + Send + Sync>> {
    if let Ok(path) = std::env::var("AGENT_CONFIG") {
        let path = PathBuf::from(path);
        info!(path = %path.display(), "Loading agent config from file");
        return Ok(AgentConfig::from_file(&path)?);
    }

    let mut config = AgentConfig::default();
    // A hostname makes a far better default agent name than a shared constant
    // would: several agents registering as "oxide-agent" is a name collision
    // waiting to happen.
    config.name = std::env::var("AGENT_NAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| config.name.clone());
    config.nats_url = env_or("NATS_URL", &config.nats_url);
    if let Ok(labels) = std::env::var("AGENT_LABELS") {
        config.labels = labels
            .split(',')
            .map(str::trim)
            .filter(|label| !label.is_empty())
            .map(String::from)
            .collect();
    }
    if let Ok(max) = std::env::var("AGENT_MAX_CONCURRENT_JOBS")
        && let Ok(parsed) = max.parse()
    {
        config.max_concurrent_jobs = parsed;
    }
    if let Ok(dir) = std::env::var("AGENT_WORKSPACE_DIR") {
        config.workspace_dir = PathBuf::from(dir);
    }
    Ok(config)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,oxide=debug".into()),
        )
        .init();

    let config = load_config()?;
    let database_url = env_or(
        "DATABASE_URL",
        "postgres://oxide:oxide_dev_password@localhost:5432/oxide",
    );

    info!(
        name = %config.name,
        nats = %config.nats_url,
        workspace = %config.workspace_dir.display(),
        max_concurrent_jobs = config.max_concurrent_jobs,
        "Starting agent"
    );

    // The workspace has to exist before the first job lands, not after.
    if let Err(e) = std::fs::create_dir_all(&config.workspace_dir) {
        warn!(
            dir = %config.workspace_dir.display(),
            error = %e,
            "Could not create the workspace directory"
        );
    }

    let database = Database::connect(&database_url).await?;
    let repository = Arc::new(PgAgentRepository::new(database.pool().clone()));
    let event_bus = Arc::new(NatsEventBus::connect(&config.nats_url).await?);

    let mut agent = BuildAgent::new(config, event_bus, repository);

    // `start` registers the agent and spawns its heartbeat, then returns — it
    // does not block, and it does not yet consume jobs (see #53). So the
    // process has to park until it is asked to stop; otherwise it would
    // register, exit immediately, and take its heartbeat with it.
    agent.start().await?;
    info!("Agent registered and sending heartbeats; waiting for shutdown");

    shutdown_signal().await;
    info!("Shutting down");

    Ok(())
}

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
        _ = ctrl_c => {}
        _ = terminate => {}
    }
}
