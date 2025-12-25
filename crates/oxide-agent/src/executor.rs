//! Job execution logic.

use oxide_core::Result;
use oxide_core::events::{Event, StageCompletedPayload, StageStartedPayload};
use oxide_core::ids::{AgentId, PipelineId, RunId};
use oxide_core::pipeline::StageDefinition;
use oxide_core::ports::EventBus;
use oxide_core::run::StageStatus;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tracing::{error, info, warn};

/// Executes jobs assigned to this agent.
pub struct JobExecutor {
    agent_id: AgentId,
    workspace_dir: PathBuf,
    event_bus: Arc<dyn EventBus>,
}

/// A job to execute.
#[derive(Debug, Clone)]
pub struct Job {
    pub run_id: RunId,
    pub pipeline_id: PipelineId,
    pub pipeline_name: String,
    pub stage: StageDefinition,
    pub stage_index: u32,
    pub variables: std::collections::HashMap<String, String>,
}

/// Result of job execution.
#[derive(Debug)]
pub struct JobResult {
    pub run_id: RunId,
    pub stage_name: String,
    pub success: bool,
    pub duration_ms: u64,
    pub error: Option<String>,
}

impl JobExecutor {
    pub fn new(agent_id: AgentId, workspace_dir: PathBuf, event_bus: Arc<dyn EventBus>) -> Self {
        Self {
            agent_id,
            workspace_dir,
            event_bus,
        }
    }

    /// Get the agent ID.
    pub fn agent_id(&self) -> AgentId {
        self.agent_id
    }

    /// Execute a job.
    pub async fn execute(&self, job: Job) -> Result<JobResult> {
        let start = std::time::Instant::now();
        let step_count = job.stage.steps.len() as u32;

        info!(
            run_id = %job.run_id,
            stage = %job.stage.name,
            "Starting job execution"
        );

        // Set up workspace
        let workspace = self.setup_workspace(&job).await?;

        // Publish stage started event
        self.publish_stage_started(&job, step_count).await?;

        // Execute steps
        let mut success = true;
        let mut error_msg = None;
        let mut steps_passed = 0u32;
        let mut steps_failed = 0u32;

        for (idx, step) in job.stage.steps.iter().enumerate() {
            info!(
                run_id = %job.run_id,
                step = %step.name,
                index = idx,
                "Executing step"
            );

            // Execute step (simplified - would delegate to oxide-runner)
            if let Some(ref cmd) = step.run {
                match self.run_command(cmd, &workspace).await {
                    Ok(_) => {
                        info!(step = %step.name, "Step completed successfully");
                        steps_passed += 1;
                    }
                    Err(e) => {
                        error!(step = %step.name, error = %e, "Step failed");
                        steps_failed += 1;
                        use oxide_core::pipeline::BooleanOrExpression;
                        let continue_on_error = match &step.continue_on_error {
                            Some(BooleanOrExpression::Boolean(b)) => *b,
                            Some(BooleanOrExpression::Expression(s)) => {
                                // Simplified interpolation for agent (TODO: full context support)
                                s == "true"
                            }
                            None => false,
                        };

                        if !continue_on_error {
                            success = false;
                            error_msg = Some(e.to_string());
                            break;
                        }
                    }
                }
            } else {
                steps_passed += 1;
            }
        }

        let duration_ms = start.elapsed().as_millis() as u64;

        // Publish stage completed event
        self.publish_stage_completed(&job, success, duration_ms, steps_passed, steps_failed)
            .await?;

        // Cleanup workspace
        self.cleanup_workspace(&workspace).await?;

        Ok(JobResult {
            run_id: job.run_id,
            stage_name: job.stage.name.clone(),
            success,
            duration_ms,
            error: error_msg,
        })
    }

    async fn setup_workspace(&self, job: &Job) -> Result<PathBuf> {
        let workspace = self
            .workspace_dir
            .join(job.run_id.to_string())
            .join(&job.stage.name);

        fs::create_dir_all(&workspace).await.map_err(|e| {
            oxide_core::Error::Internal(format!("Failed to create workspace: {}", e))
        })?;

        info!(path = %workspace.display(), "Workspace created");
        Ok(workspace)
    }

    async fn cleanup_workspace(&self, workspace: &PathBuf) -> Result<()> {
        if workspace.exists()
            && let Err(e) = fs::remove_dir_all(workspace).await
        {
            warn!(path = %workspace.display(), error = %e, "Failed to cleanup workspace");
        }
        Ok(())
    }

    async fn run_command(&self, cmd: &str, workspace: &PathBuf) -> Result<()> {
        let output = tokio::process::Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .current_dir(workspace)
            .output()
            .await
            .map_err(|e| oxide_core::Error::Internal(format!("Command execution failed: {}", e)))?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(oxide_core::Error::Internal(format!(
                "Command failed with exit code {:?}: {}",
                output.status.code(),
                stderr
            )))
        }
    }

    async fn publish_stage_started(&self, job: &Job, step_count: u32) -> Result<()> {
        let event = Event::StageStarted(StageStartedPayload {
            run_id: job.run_id,
            stage_name: job.stage.name.clone(),
            stage_index: job.stage_index,
            step_count,
            started_at: chrono::Utc::now(),
        });
        self.event_bus.publish(event).await
    }

    async fn publish_stage_completed(
        &self,
        job: &Job,
        success: bool,
        duration_ms: u64,
        steps_passed: u32,
        steps_failed: u32,
    ) -> Result<()> {
        let status = if success {
            StageStatus::Success
        } else {
            StageStatus::Failure
        };

        let event = Event::StageCompleted(StageCompletedPayload {
            run_id: job.run_id,
            stage_name: job.stage.name.clone(),
            stage_index: job.stage_index,
            status,
            duration_ms,
            steps_passed,
            steps_failed,
            completed_at: chrono::Utc::now(),
        });
        self.event_bus.publish(event).await
    }
}
