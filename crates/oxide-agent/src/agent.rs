//! Main agent implementation.

use crate::config::AgentConfig;
use crate::executor::{Job, JobExecutor};
use crate::heartbeat::HeartbeatService;
use oxide_core::Result;
use oxide_core::agent::{Agent, AgentStatus, DisconnectReason};
use oxide_core::events::{AgentDisconnectedPayload, AgentRegisteredPayload, Event};
use oxide_core::ids::{AgentId, RunId};
use oxide_core::ports::{AgentRepository, EventBus};
use std::sync::Arc;
use tokio::sync::{Semaphore, watch};
use tracing::{error, info, warn};

/// The build agent.
pub struct BuildAgent {
    config: AgentConfig,
    agent_id: AgentId,
    event_bus: Arc<dyn EventBus>,
    repository: Arc<dyn AgentRepository>,
    executor: JobExecutor,
    status_tx: watch::Sender<AgentStatus>,
    status_rx: watch::Receiver<AgentStatus>,
    current_run_tx: watch::Sender<Option<RunId>>,
    current_run_rx: watch::Receiver<Option<RunId>>,
    shutdown_tx: watch::Sender<bool>,
    shutdown_rx: watch::Receiver<bool>,
    job_semaphore: Arc<Semaphore>,
}

impl BuildAgent {
    /// Create a new build agent.
    pub fn new(
        config: AgentConfig,
        event_bus: Arc<dyn EventBus>,
        repository: Arc<dyn AgentRepository>,
    ) -> Self {
        let agent_id = AgentId::default();
        let (status_tx, status_rx) = watch::channel(AgentStatus::Offline);
        let (current_run_tx, current_run_rx) = watch::channel(None);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let job_semaphore = Arc::new(Semaphore::new(config.max_concurrent_jobs as usize));

        let executor = JobExecutor::new(
            agent_id,
            config.workspace_dir.clone(),
            Arc::clone(&event_bus),
        );

        Self {
            config,
            agent_id,
            event_bus,
            repository,
            executor,
            status_tx,
            status_rx,
            current_run_tx,
            current_run_rx,
            shutdown_tx,
            shutdown_rx,
            job_semaphore,
        }
    }

    /// Start the agent.
    pub async fn start(&mut self) -> Result<()> {
        info!(name = %self.config.name, "Starting build agent");

        // Register with the scheduler
        self.register().await?;

        // Update status to idle
        let _ = self.status_tx.send(AgentStatus::Idle);

        // Start heartbeat service
        let heartbeat = HeartbeatService::new(
            self.agent_id,
            Arc::clone(&self.event_bus),
            self.config.heartbeat_interval_secs,
            self.status_rx.clone(),
            self.current_run_rx.clone(),
        );

        let shutdown_rx = self.shutdown_rx.clone();
        tokio::spawn(async move {
            heartbeat.run(shutdown_rx).await;
        });

        info!(agent_id = %self.agent_id, "Agent started and ready for jobs");

        Ok(())
    }

    /// Register the agent with the scheduler.
    async fn register(&mut self) -> Result<()> {
        let version = env!("CARGO_PKG_VERSION").to_string();

        let agent = Agent {
            id: self.agent_id,
            name: self.config.name.clone(),
            labels: self.config.labels.clone(),
            capabilities: self.config.capabilities.clone(),
            status: AgentStatus::Idle,
            os: AgentConfig::detect_os(),
            arch: AgentConfig::detect_arch(),
            version: Some(version.clone()),
            max_concurrent_jobs: self.config.max_concurrent_jobs,
            current_run_id: None,
            system_metrics: None,
            registered_at: chrono::Utc::now(),
            last_heartbeat_at: Some(chrono::Utc::now()),
        };

        // Register in repository
        let assigned_id = self.repository.register(&agent).await?;
        self.agent_id = assigned_id;

        info!(agent_id = %self.agent_id, name = %self.config.name, "Agent registered");

        // Publish registration event
        let event = Event::AgentRegistered(AgentRegisteredPayload {
            agent_id: self.agent_id,
            name: self.config.name.clone(),
            labels: self.config.labels.clone(),
            version: Some(version),
            registered_at: agent.registered_at,
        });

        self.event_bus.publish(event).await?;

        Ok(())
    }

    /// Execute a job.
    pub async fn execute_job(&self, job: Job) -> Result<()> {
        // Acquire semaphore permit
        let permit = self
            .job_semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| oxide_core::Error::Internal("Job semaphore closed".to_string()))?;

        // Update status to busy
        let _ = self.status_tx.send(AgentStatus::Busy);
        let _ = self.current_run_tx.send(Some(job.run_id));

        let result = self.executor.execute(job).await;

        // Release permit
        drop(permit);

        // Clear current run
        let _ = self.current_run_tx.send(None);

        // Check if we're idle now
        if self.job_semaphore.available_permits() == self.config.max_concurrent_jobs as usize {
            let _ = self.status_tx.send(AgentStatus::Idle);
        }

        match result {
            Ok(job_result) => {
                if job_result.success {
                    info!(
                        run_id = %job_result.run_id,
                        stage = %job_result.stage_name,
                        duration_ms = job_result.duration_ms,
                        "Job completed successfully"
                    );
                } else {
                    warn!(
                        run_id = %job_result.run_id,
                        stage = %job_result.stage_name,
                        error = ?job_result.error,
                        "Job failed"
                    );
                }
                Ok(())
            }
            Err(e) => {
                error!(error = %e, "Job execution error");
                Err(e)
            }
        }
    }

    /// Initiate graceful shutdown.
    pub async fn shutdown(&self) -> Result<()> {
        info!("Initiating graceful shutdown");

        // Signal shutdown
        let _ = self.shutdown_tx.send(true);

        // Enter drain mode
        let _ = self.status_tx.send(AgentStatus::Draining);

        // Wait for all jobs to complete
        info!("Waiting for in-progress jobs to complete...");
        let _ = self
            .job_semaphore
            .acquire_many(self.config.max_concurrent_jobs)
            .await;

        // Deregister from scheduler
        self.repository.deregister(self.agent_id).await?;

        // Publish disconnected event
        let event = Event::AgentDisconnected(AgentDisconnectedPayload {
            agent_id: self.agent_id,
            reason: DisconnectReason::Graceful,
            last_heartbeat_at: Some(chrono::Utc::now()),
            disconnected_at: chrono::Utc::now(),
        });
        let _ = self.event_bus.publish(event).await;

        info!(agent_id = %self.agent_id, "Agent shutdown complete");
        Ok(())
    }

    /// Get the agent ID.
    pub fn id(&self) -> AgentId {
        self.agent_id
    }

    /// Get the current status.
    pub fn status(&self) -> AgentStatus {
        *self.status_rx.borrow()
    }
}
