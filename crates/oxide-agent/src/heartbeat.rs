//! Heartbeat loop for periodic health reporting.

use oxide_core::agent::{AgentStatus, SystemMetrics};
use oxide_core::events::{AgentHeartbeatPayload, Event};
use oxide_core::ids::{AgentId, RunId};
use oxide_core::ports::EventBus;
use std::sync::Arc;
use sysinfo::System;
use tokio::sync::watch;
use tokio::time::{Duration, interval};
use tracing::{debug, error, info};

/// Heartbeat service that periodically reports agent health.
pub struct HeartbeatService {
    agent_id: AgentId,
    event_bus: Arc<dyn EventBus>,
    interval_secs: u64,
    status_rx: watch::Receiver<AgentStatus>,
    current_run_rx: watch::Receiver<Option<RunId>>,
}

impl HeartbeatService {
    pub fn new(
        agent_id: AgentId,
        event_bus: Arc<dyn EventBus>,
        interval_secs: u64,
        status_rx: watch::Receiver<AgentStatus>,
        current_run_rx: watch::Receiver<Option<RunId>>,
    ) -> Self {
        Self {
            agent_id,
            event_bus,
            interval_secs,
            status_rx,
            current_run_rx,
        }
    }

    /// Run the heartbeat loop until shutdown.
    pub async fn run(&self, mut shutdown: watch::Receiver<bool>) {
        let mut ticker = interval(Duration::from_secs(self.interval_secs));
        let mut sys = System::new_all();

        info!(
            agent_id = %self.agent_id,
            interval_secs = self.interval_secs,
            "Starting heartbeat service"
        );

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    self.send_heartbeat(&mut sys).await;
                }
                _ = shutdown.changed() => {
                    if *shutdown.borrow() {
                        info!("Heartbeat service shutting down");
                        break;
                    }
                }
            }
        }
    }

    async fn send_heartbeat(&self, sys: &mut System) {
        sys.refresh_all();

        let load = System::load_average();
        let metrics = SystemMetrics {
            cpu_percent: sys.global_cpu_usage() as f64,
            memory_total_bytes: sys.total_memory(),
            memory_used_bytes: sys.used_memory(),
            disk_total_bytes: 0,
            disk_used_bytes: 0,
            load_average: (load.one, load.five, load.fifteen),
        };

        let status = *self.status_rx.borrow();
        let current_run_id = *self.current_run_rx.borrow();

        let event = Event::AgentHeartbeat(AgentHeartbeatPayload {
            agent_id: self.agent_id,
            status,
            current_run_id,
            system_metrics: Some(metrics),
            timestamp: chrono::Utc::now(),
        });

        if let Err(e) = self.event_bus.publish(event).await {
            error!(error = %e, "Failed to send heartbeat");
        } else {
            debug!(agent_id = %self.agent_id, "Heartbeat sent");
        }
    }
}
