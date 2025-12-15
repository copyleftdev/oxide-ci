//! Main scheduler orchestration.

use crate::agents::AgentMatcher;
use crate::dag::{DagBuilder, PipelineDag};
use crate::matrix::MatrixExpander;
use crate::queue::{Priority, QueueManager, QueuedJob};
use crate::triggers::{TriggerEvent, TriggerMatcher};

use oxide_core::Result;
use oxide_core::agent::Capability;
use oxide_core::events::{Event, RunQueuedPayload};
use oxide_core::ids::{PipelineId, RunId};
use oxide_core::pipeline::{EnvironmentType, PipelineDefinition};
use oxide_core::ports::{AgentRepository, EventBus, PipelineRepository, RunRepository};
use oxide_core::run::{Run, RunStatus};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// The main scheduler service.
#[allow(dead_code)]
pub struct Scheduler {
    pipelines: Arc<dyn PipelineRepository>,
    runs: Arc<dyn RunRepository>,
    agents: Arc<dyn AgentRepository>,
    event_bus: Arc<dyn EventBus>,
    trigger_matcher: TriggerMatcher,
    dag_builder: DagBuilder,
    matrix_expander: MatrixExpander,
    agent_matcher: AgentMatcher,
    queue: Arc<RwLock<QueueManager>>,
    active_runs: Arc<RwLock<HashMap<RunId, RunState>>>,
}

/// State of an active run.
#[derive(Debug)]
struct RunState {
    pipeline_id: PipelineId,
    dag: PipelineDag,
    completed_stages: Vec<String>,
    failed_stages: Vec<String>,
}

impl Scheduler {
    pub fn new(
        pipelines: Arc<dyn PipelineRepository>,
        runs: Arc<dyn RunRepository>,
        agents: Arc<dyn AgentRepository>,
        event_bus: Arc<dyn EventBus>,
    ) -> Self {
        Self {
            pipelines: pipelines.clone(),
            runs,
            agents: agents.clone(),
            event_bus,
            trigger_matcher: TriggerMatcher::new(),
            dag_builder: DagBuilder::new(),
            matrix_expander: MatrixExpander::new(),
            agent_matcher: AgentMatcher::new(agents),
            queue: Arc::new(RwLock::new(QueueManager::new())),
            active_runs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Handle a trigger event and potentially start pipeline runs.
    pub async fn handle_trigger(&self, event: TriggerEvent) -> Result<Vec<RunId>> {
        let pipelines = self.pipelines.list(100, 0).await?;
        let mut triggered_runs = Vec::new();

        for pipeline in pipelines {
            if self.trigger_matcher.matches(&pipeline.definition, &event) {
                let run_id = self.start_run(pipeline.id, &pipeline.definition).await?;
                triggered_runs.push(run_id);
            }
        }

        Ok(triggered_runs)
    }

    /// Start a new pipeline run.
    pub async fn start_run(
        &self,
        pipeline_id: PipelineId,
        definition: &PipelineDefinition,
    ) -> Result<RunId> {
        // Build the DAG
        let dag = self
            .dag_builder
            .build(definition)
            .map_err(|e| oxide_core::Error::Internal(e.to_string()))?;

        // Get next run number
        let run_number = self.runs.next_run_number(pipeline_id).await?;

        // Create the run
        let run = Run {
            id: RunId::default(),
            pipeline_id,
            pipeline_name: definition.name.clone(),
            run_number,
            status: RunStatus::Queued,
            trigger: oxide_core::run::TriggerInfo {
                trigger_type: oxide_core::pipeline::TriggerType::Api,
                triggered_by: None,
                source: None,
            },
            git_ref: None,
            git_sha: None,
            variables: definition.variables.clone(),
            stages: vec![],
            queued_at: chrono::Utc::now(),
            started_at: None,
            completed_at: None,
            duration_ms: None,
            billable_minutes: None,
        };

        let run_id = self.runs.create(&run).await?;

        // Store run state
        {
            let mut active = self.active_runs.write().await;
            active.insert(
                run_id,
                RunState {
                    pipeline_id,
                    dag,
                    completed_stages: vec![],
                    failed_stages: vec![],
                },
            );
        }

        // Queue root stages
        self.queue_ready_stages(run_id).await?;

        // Publish event
        let event = Event::RunQueued(RunQueuedPayload {
            run_id,
            pipeline_id,
            pipeline_name: definition.name.clone(),
            run_number,
            trigger: oxide_core::pipeline::TriggerType::Api,
            git_ref: None,
            git_sha: None,
            queued_at: chrono::Utc::now(),
            queued_by: None,
            license_id: None,
        });
        self.event_bus.publish(event).await?;

        Ok(run_id)
    }

    /// Queue stages that are ready to run.
    async fn queue_ready_stages(&self, run_id: RunId) -> Result<()> {
        let active = self.active_runs.read().await;
        let state = match active.get(&run_id) {
            Some(s) => s,
            None => return Ok(()),
        };

        let mut queue = self.queue.write().await;

        for node in state.dag.stages() {
            if state.completed_stages.contains(&node.name) {
                continue;
            }
            if state.failed_stages.contains(&node.name) {
                continue;
            }
            if !state.dag.is_ready(&node.name, &state.completed_stages) {
                continue;
            }

            // Check if this stage has a matrix
            if let Some(expansion) = self.matrix_expander.expand(&node.definition) {
                // Queue each matrix job
                for job in expansion.jobs {
                    queue.enqueue(QueuedJob {
                        run_id,
                        pipeline_id: state.pipeline_id,
                        stage_name: node.name.clone(),
                        job_index: Some(job.index),
                        priority: Priority::Normal,
                        queued_at: chrono::Utc::now(),
                        labels: node
                            .definition
                            .agent
                            .as_ref()
                            .map(|a| a.labels.clone())
                            .unwrap_or_default(),
                        concurrency_group: None,
                    });
                }
            } else {
                // Queue single job
                queue.enqueue(QueuedJob {
                    run_id,
                    pipeline_id: state.pipeline_id,
                    stage_name: node.name.clone(),
                    job_index: None,
                    priority: Priority::Normal,
                    queued_at: chrono::Utc::now(),
                    labels: node
                        .definition
                        .agent
                        .as_ref()
                        .map(|a| a.labels.clone())
                        .unwrap_or_default(),
                    concurrency_group: None,
                });
            }
        }

        Ok(())
    }

    /// Process the queue and assign jobs to agents.
    pub async fn process_queue(&self) -> Result<Vec<(QueuedJob, oxide_core::agent::Agent)>> {
        let mut assigned = Vec::new();
        let mut queue = self.queue.write().await;

        while let Some(job) = queue.dequeue() {
            // Determine required capabilities from stage definition
            let capabilities = self.get_required_capabilities(&job).await;

            // Find an available agent
            if let Some(agent) = self
                .agent_matcher
                .find_available(&job.labels, &capabilities)
                .await?
            {
                assigned.push((job, agent));
            } else {
                // No agent available, put back in queue
                queue.enqueue(job);
                break;
            }
        }

        Ok(assigned)
    }

    async fn get_required_capabilities(&self, job: &QueuedJob) -> Vec<Capability> {
        // Look up stage definition and determine required capabilities
        let active = self.active_runs.read().await;
        if let Some(state) = active.get(&job.run_id) {
            for node in state.dag.stages() {
                if node.name == job.stage_name
                    && let Some(env) = &node.definition.environment
                {
                    return match env.env_type {
                        EnvironmentType::Container => vec![Capability::Docker],
                        EnvironmentType::Firecracker => vec![Capability::Firecracker],
                        EnvironmentType::Nix => vec![Capability::Nix],
                        EnvironmentType::Host => vec![],
                    };
                }
            }
        }
        vec![Capability::Docker] // Default to Docker
    }

    /// Mark a stage as completed.
    pub async fn stage_completed(
        &self,
        run_id: RunId,
        stage_name: &str,
        success: bool,
    ) -> Result<()> {
        {
            let mut active = self.active_runs.write().await;
            if let Some(state) = active.get_mut(&run_id) {
                if success {
                    state.completed_stages.push(stage_name.to_string());
                } else {
                    state.failed_stages.push(stage_name.to_string());
                }
            }
        }

        // Queue newly ready stages
        self.queue_ready_stages(run_id).await?;

        // Check if run is complete
        self.check_run_complete(run_id).await?;

        Ok(())
    }

    async fn check_run_complete(&self, run_id: RunId) -> Result<()> {
        let active = self.active_runs.read().await;
        if let Some(state) = active.get(&run_id) {
            let total_stages = state.dag.stages().len();
            let done = state.completed_stages.len() + state.failed_stages.len();

            if done == total_stages {
                drop(active);

                let status = if self
                    .active_runs
                    .read()
                    .await
                    .get(&run_id)
                    .map(|s| s.failed_stages.is_empty())
                    .unwrap_or(true)
                {
                    RunStatus::Success
                } else {
                    RunStatus::Failure
                };

                if let Some(mut run) = self.runs.get(run_id).await? {
                    run.status = status;
                    run.completed_at = Some(chrono::Utc::now());
                    if let Some(started) = run.started_at {
                        run.duration_ms =
                            Some((chrono::Utc::now() - started).num_milliseconds() as u64);
                    }
                    self.runs.update(&run).await?;
                }

                // Remove from active runs
                self.active_runs.write().await.remove(&run_id);
            }
        }

        Ok(())
    }

    /// Get the current queue length.
    pub async fn queue_length(&self) -> usize {
        self.queue.read().await.len()
    }
}
