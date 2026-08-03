//! The `oxide-scheduler` service.
//!
//! Matches queued work to registered agents and advances runs as stages
//! finish. It holds no state a restart cannot rebuild from the database and
//! the event bus.

use oxide_db::{Database, PgAgentRepository, PgPipelineRepository, PgRunRepository};
use oxide_nats::NatsEventBus;
use oxide_scheduler::{Scheduler, SchedulerService};
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};

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
    let poll_interval = Duration::from_millis(
        env_or("SCHEDULER_POLL_INTERVAL_MS", "500")
            .parse()
            .unwrap_or(500),
    );

    let database = Database::connect(&database_url).await?;
    info!("Database ready");

    let event_bus = Arc::new(NatsEventBus::connect(&nats_url).await?);
    info!("Event bus ready");

    let pool = database.pool().clone();
    let pipelines = Arc::new(PgPipelineRepository::new(pool.clone()));
    let scheduler = Arc::new(Scheduler::new(
        pipelines.clone(),
        Arc::new(PgRunRepository::new(pool.clone())),
        Arc::new(PgAgentRepository::new(pool)),
        event_bus.clone(),
    ));

    let service =
        SchedulerService::new(scheduler, pipelines, event_bus).with_poll_interval(poll_interval);

    service.run(shutdown_signal()).await;
    info!("Scheduler stopped");
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
