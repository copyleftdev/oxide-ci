//! The scheduler as a running service.
//!
//! [`Scheduler`](crate::Scheduler) knows how to match a queued job to an agent
//! and how to advance a DAG. This drives both on a loop: dispatch what the
//! queue can place, and react to what comes back.
//!
//! Every dispatch publishes why it happened — which labels matched, how long
//! the job waited, which attempt this is — so a slow or surprising placement
//! can be explained from the event stream rather than from scheduler logs that
//! nobody kept.

use crate::Scheduler;
use crate::queue::QueuedJob;
use futures::StreamExt;
use oxide_core::Result;
use oxide_core::events::{Event, JobDispatchedPayload, JobRejectionReason};
use oxide_core::ids::JobId;
use oxide_core::ports::{EventBus, PipelineRepository};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

/// How often the queue is examined when nothing else wakes the loop.
const DEFAULT_POLL_INTERVAL: Duration = Duration::from_millis(500);

pub struct SchedulerService {
    scheduler: Arc<Scheduler>,
    pipelines: Arc<dyn PipelineRepository>,
    event_bus: Arc<dyn EventBus>,
    poll_interval: Duration,
    /// Jobs handed to an agent and not yet resolved, so a rejection can put the
    /// original job back rather than losing it.
    in_flight: Arc<Mutex<HashMap<JobId, QueuedJob>>>,
}

impl SchedulerService {
    pub fn new(
        scheduler: Arc<Scheduler>,
        pipelines: Arc<dyn PipelineRepository>,
        event_bus: Arc<dyn EventBus>,
    ) -> Self {
        Self {
            scheduler,
            pipelines,
            event_bus,
            poll_interval: DEFAULT_POLL_INTERVAL,
            in_flight: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn with_poll_interval(mut self, interval: Duration) -> Self {
        self.poll_interval = interval;
        self
    }

    /// Run until `shutdown` resolves.
    pub async fn run(&self, shutdown: impl std::future::Future<Output = ()> + Send) {
        info!(
            poll_interval_ms = self.poll_interval.as_millis() as u64,
            "Scheduler service started"
        );

        let replies = self.subscribe_to_replies().await;
        tokio::pin!(shutdown);
        tokio::pin!(replies);

        let mut ticker = tokio::time::interval(self.poll_interval);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = &mut shutdown => {
                    info!("Scheduler service stopping");
                    return;
                }
                _ = ticker.tick() => {
                    if let Err(e) = self.dispatch_ready_jobs().await {
                        error!(error = %e, "Dispatch pass failed");
                    }
                }
                Some(event) = replies.next() => {
                    self.handle_event(event).await;
                }
            }
        }
    }

    /// Subscribe to everything the scheduler must react to.
    ///
    /// A subscription failure is not fatal — dispatching still works — but it
    /// is loud, because a scheduler that cannot see stage completions will
    /// stall every multi-stage run.
    async fn subscribe_to_replies(&self) -> impl futures::Stream<Item = Event> + Send + use<> {
        let mut streams = Vec::new();
        for pattern in [
            "run.queued.*",
            "run.*.job.rejected",
            "run.*.stage.*.completed",
        ] {
            match self.event_bus.subscribe(pattern).await {
                Ok(stream) => streams.push(stream),
                Err(e) => error!(pattern, error = %e, "Could not subscribe; runs may stall"),
            }
        }
        futures::stream::select_all(streams).filter_map(|event| async move {
            match event {
                Ok(event) => Some(event),
                Err(e) => {
                    warn!(error = %e, "Dropped a malformed event");
                    None
                }
            }
        })
    }

    async fn handle_event(&self, event: Event) {
        match event {
            Event::JobRejected(payload) => {
                let job = self.in_flight.lock().await.remove(&payload.job_id);

                // The typed reason is what makes this actionable: capacity
                // pressure and an impossible requirement need different
                // responses, and retrying the second forever hides it.
                match (payload.retryable, job) {
                    (true, Some(job)) => {
                        warn!(
                            job_id = %payload.job_id,
                            agent_id = %payload.agent_id,
                            reason = ?payload.reason,
                            detail = payload.detail.as_deref().unwrap_or(""),
                            "Job rejected; requeuing"
                        );
                        self.scheduler.requeue(job).await;
                    }
                    (false, _) => {
                        error!(
                            job_id = %payload.job_id,
                            agent_id = %payload.agent_id,
                            reason = ?payload.reason,
                            detail = payload.detail.as_deref().unwrap_or(""),
                            "Job rejected and cannot be retried; it will not be redispatched"
                        );
                    }
                    (true, None) => {
                        warn!(
                            job_id = %payload.job_id,
                            reason = ?payload.reason,
                            "Rejection for a job this scheduler did not dispatch"
                        );
                    }
                }
            }
            Event::RunQueued(payload) => {
                // Another process created this run; the scheduler has to adopt
                // it before any of its stages can be placed.
                match self.pipelines.get(payload.pipeline_id).await {
                    Ok(Some(pipeline)) => {
                        if let Err(e) = self
                            .scheduler
                            .adopt_run(payload.run_id, payload.pipeline_id, &pipeline.definition)
                            .await
                        {
                            error!(run_id = %payload.run_id, error = %e, "Could not adopt the run");
                        } else {
                            info!(
                                run_id = %payload.run_id,
                                pipeline = %pipeline.name,
                                "Adopted run; queueing ready stages"
                            );
                        }
                    }
                    Ok(None) => error!(
                        pipeline_id = %payload.pipeline_id,
                        "Run queued for a pipeline that does not exist"
                    ),
                    Err(e) => error!(error = %e, "Could not load the pipeline for a queued run"),
                }
            }
            Event::StageCompleted(payload) => {
                let success = matches!(payload.status, oxide_core::run::StageStatus::Success);
                if let Err(e) = self
                    .scheduler
                    .stage_completed(payload.run_id, &payload.stage_name, success)
                    .await
                {
                    error!(
                        run_id = %payload.run_id,
                        stage = %payload.stage_name,
                        error = %e,
                        "Could not advance the run"
                    );
                }
            }
            _ => {}
        }
    }

    /// Place whatever the queue can place right now.
    async fn dispatch_ready_jobs(&self) -> Result<()> {
        let assignments = self.scheduler.process_queue().await?;
        if assignments.is_empty() {
            return Ok(());
        }

        for (job, agent) in assignments {
            let dispatched_at = chrono::Utc::now();
            let job_id = JobId::new();

            // The steps come from the pipeline definition rather than from the
            // queue entry, so an agent never has to resolve anything itself.
            let pipeline = match self.pipelines.get(job.pipeline_id).await {
                Ok(Some(pipeline)) => pipeline,
                Ok(None) => {
                    error!(
                        pipeline_id = %job.pipeline_id,
                        stage = %job.stage_name,
                        "Pipeline vanished between queueing and dispatch; dropping the job"
                    );
                    continue;
                }
                Err(e) => {
                    error!(pipeline_id = %job.pipeline_id, error = %e, "Could not load the pipeline; requeuing");
                    self.scheduler.requeue(job).await;
                    continue;
                }
            };

            let Some(stage) = pipeline
                .definition
                .stages
                .iter()
                .find(|stage| stage.name == job.stage_name)
            else {
                error!(
                    pipeline = %pipeline.name,
                    stage = %job.stage_name,
                    "Stage is not in the pipeline definition; dropping the job"
                );
                continue;
            };

            // Why this agent: the labels the job asked for that it has.
            let matched_labels: Vec<String> = job
                .labels
                .iter()
                .filter(|label| agent.labels.contains(label))
                .cloned()
                .collect();

            let queue_wait_ms = (dispatched_at - job.queued_at).num_milliseconds().max(0) as u64;

            let payload = JobDispatchedPayload {
                job_id,
                run_id: job.run_id,
                pipeline_id: job.pipeline_id,
                pipeline_name: pipeline.name.clone(),
                stage_name: job.stage_name.clone(),
                job_index: job.job_index.map(|index| index as u32),
                agent_id: agent.id,
                agent_name: agent.name.clone(),
                matched_labels,
                steps: stage.steps.clone(),
                variables: pipeline.definition.variables.clone(),
                attempt: 1,
                queue_wait_ms,
                timeout_minutes: stage.timeout_minutes,
                queued_at: job.queued_at,
                dispatched_at,
            };

            self.in_flight.lock().await.insert(job_id, job);

            debug!(
                %job_id,
                agent = %agent.name,
                stage = %payload.stage_name,
                queue_wait_ms,
                "Dispatching"
            );

            if let Err(e) = self.event_bus.publish(Event::JobDispatched(payload)).await {
                // Publishing failed, so no agent will ever see this job.
                error!(%job_id, error = %e, "Dispatch publish failed; requeuing");
                if let Some(job) = self.in_flight.lock().await.remove(&job_id) {
                    self.scheduler.requeue(job).await;
                }
            }
        }

        Ok(())
    }
}

/// Reasons the service treats as permanent, for documentation of intent.
pub fn is_retryable(reason: JobRejectionReason) -> bool {
    match reason {
        JobRejectionReason::AtCapacity
        | JobRejectionReason::Draining
        | JobRejectionReason::WorkspaceUnavailable => true,
        JobRejectionReason::MissingCapability | JobRejectionReason::UnknownJob => false,
    }
}
