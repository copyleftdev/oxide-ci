//! Test fixtures for creating sample data.

use chrono::Utc;
use oxide_core::ids::{PipelineId, RunId, StageId, StepId};
use oxide_core::pipeline::{
    Pipeline, PipelineDefinition, StageDefinition, StepDefinition, TriggerConfig, TriggerType,
};
use oxide_core::run::{Run, RunStatus, Stage, StageStatus, Step, StepStatus, TriggerInfo};
use std::collections::HashMap;

/// Factory for creating test pipelines.
pub struct PipelineFixture;

impl PipelineFixture {
    /// Create a simple pipeline with one stage and one step.
    pub fn simple() -> Pipeline {
        Pipeline {
            id: PipelineId::new(),
            name: "test-pipeline".to_string(),
            definition: PipelineDefinition {
                version: "1".to_string(),
                name: "test-pipeline".to_string(),
                description: Some("A simple test pipeline".to_string()),
                triggers: vec![TriggerConfig::Explicit {
                    trigger_type: TriggerType::Manual,
                    branches: vec![],
                    paths: vec![],
                    paths_ignore: vec![],
                    tags: vec![],
                    cron: None,
                    timezone: None,
                }],
                variables: HashMap::new(),
                stages: vec![StageDefinition {
                    name: "build".to_string(),
                    display_name: Some("Build".to_string()),
                    depends_on: vec![],
                    condition: None,
                    environment: None,
                    variables: HashMap::new(),
                    steps: vec![Self::echo_step("hello")],
                    parallel: false,
                    timeout_minutes: Some(30),
                    retry: None,
                    agent: None,
                    matrix: None,
                }],
                cache: None,
                artifacts: None,
                timeout_minutes: 60,
                concurrency: None,
            },
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// Create a pipeline with multiple sequential stages.
    pub fn multi_stage() -> Pipeline {
        let mut pipeline = Self::simple();
        pipeline.name = "multi-stage-pipeline".to_string();
        pipeline.definition.name = "multi-stage-pipeline".to_string();
        pipeline.definition.stages = vec![
            StageDefinition {
                name: "build".to_string(),
                display_name: Some("Build".to_string()),
                depends_on: vec![],
                condition: None,
                environment: None,
                variables: HashMap::new(),
                steps: vec![Self::echo_step("building")],
                parallel: false,
                timeout_minutes: Some(30),
                retry: None,
                agent: None,
                matrix: None,
            },
            StageDefinition {
                name: "test".to_string(),
                display_name: Some("Test".to_string()),
                depends_on: vec!["build".to_string()],
                condition: None,
                environment: None,
                variables: HashMap::new(),
                steps: vec![Self::echo_step("testing")],
                parallel: false,
                timeout_minutes: Some(30),
                retry: None,
                agent: None,
                matrix: None,
            },
            StageDefinition {
                name: "deploy".to_string(),
                display_name: Some("Deploy".to_string()),
                depends_on: vec!["test".to_string()],
                condition: None,
                environment: None,
                variables: HashMap::new(),
                steps: vec![Self::echo_step("deploying")],
                parallel: false,
                timeout_minutes: Some(30),
                retry: None,
                agent: None,
                matrix: None,
            },
        ];
        pipeline
    }

    /// Create a pipeline with parallel stages.
    pub fn parallel() -> Pipeline {
        let mut pipeline = Self::simple();
        pipeline.name = "parallel-pipeline".to_string();
        pipeline.definition.name = "parallel-pipeline".to_string();
        pipeline.definition.stages = vec![
            StageDefinition {
                name: "lint".to_string(),
                display_name: Some("Lint".to_string()),
                depends_on: vec![],
                condition: None,
                environment: None,
                variables: HashMap::new(),
                steps: vec![Self::echo_step("linting")],
                parallel: false,
                timeout_minutes: Some(10),
                retry: None,
                agent: None,
                matrix: None,
            },
            StageDefinition {
                name: "test".to_string(),
                display_name: Some("Test".to_string()),
                depends_on: vec![],
                condition: None,
                environment: None,
                variables: HashMap::new(),
                steps: vec![Self::echo_step("testing")],
                parallel: false,
                timeout_minutes: Some(30),
                retry: None,
                agent: None,
                matrix: None,
            },
            StageDefinition {
                name: "deploy".to_string(),
                display_name: Some("Deploy".to_string()),
                depends_on: vec!["lint".to_string(), "test".to_string()],
                condition: None,
                environment: None,
                variables: HashMap::new(),
                steps: vec![Self::echo_step("deploying")],
                parallel: false,
                timeout_minutes: Some(30),
                retry: None,
                agent: None,
                matrix: None,
            },
        ];
        pipeline
    }

    /// Create an echo step helper.
    fn echo_step(message: &str) -> StepDefinition {
        StepDefinition {
            with: Default::default(),
            name: message.to_string(),
            display_name: Some(message.to_string()),
            plugin: None,
            run: Some(format!("echo '{}'", message)),
            shell: "bash".to_string(),
            working_directory: None,
            environment: None,
            variables: HashMap::new(),
            secrets: vec![],
            condition: None,
            timeout_minutes: 5,
            retry: None,
            continue_on_error: None,
            outputs: vec![],
        }
    }
}

/// Factory for creating test runs.
pub struct RunFixture;

impl RunFixture {
    /// Create a queued run for a pipeline.
    pub fn queued(pipeline: &Pipeline) -> Run {
        Run {
            id: RunId::new(),
            pipeline_id: pipeline.id,
            pipeline_name: pipeline.name.clone(),
            run_number: 1,
            status: RunStatus::Queued,
            trigger: TriggerInfo {
                trigger_type: TriggerType::Manual,
                triggered_by: Some("test@example.com".to_string()),
                source: None,
            },
            git_ref: Some("refs/heads/main".to_string()),
            git_sha: Some("abc123def456".to_string()),
            variables: HashMap::new(),
            stages: pipeline
                .definition
                .stages
                .iter()
                .map(|s| Stage {
                    id: StageId::new(&s.name),
                    name: s.name.clone(),
                    display_name: s.display_name.clone(),
                    status: StageStatus::Pending,
                    steps: s
                        .steps
                        .iter()
                        .map(|step| Step {
                            id: StepId::new(&step.name),
                            name: step.name.clone(),
                            display_name: step.display_name.clone(),
                            status: StepStatus::Pending,
                            plugin: step.plugin.clone(),
                            exit_code: None,
                            outputs: HashMap::new(),
                            started_at: None,
                            completed_at: None,
                            duration_ms: None,
                        })
                        .collect(),
                    depends_on: vec![],
                    agent_id: None,
                    started_at: None,
                    completed_at: None,
                    duration_ms: None,
                })
                .collect(),
            queued_at: Utc::now(),
            started_at: None,
            completed_at: None,
            duration_ms: None,
            billable_minutes: None,
        }
    }

    /// Create a running run.
    pub fn running(pipeline: &Pipeline) -> Run {
        let mut run = Self::queued(pipeline);
        run.status = RunStatus::Running;
        run.started_at = Some(Utc::now());
        if let Some(stage) = run.stages.first_mut() {
            stage.status = StageStatus::Running;
            stage.started_at = Some(Utc::now());
            if let Some(step) = stage.steps.first_mut() {
                step.status = StepStatus::Running;
                step.started_at = Some(Utc::now());
            }
        }
        run
    }

    /// Create a completed successful run.
    pub fn success(pipeline: &Pipeline) -> Run {
        let mut run = Self::queued(pipeline);
        let now = Utc::now();
        run.status = RunStatus::Success;
        run.started_at = Some(now);
        run.completed_at = Some(now);
        run.duration_ms = Some(1000);
        run.billable_minutes = Some(0.02);
        for stage in &mut run.stages {
            stage.status = StageStatus::Success;
            stage.started_at = Some(now);
            stage.completed_at = Some(now);
            stage.duration_ms = Some(500);
            for step in &mut stage.steps {
                step.status = StepStatus::Success;
                step.started_at = Some(now);
                step.completed_at = Some(now);
                step.exit_code = Some(0);
                step.duration_ms = Some(250);
            }
        }
        run
    }

    /// Create a failed run.
    pub fn failed(pipeline: &Pipeline) -> Run {
        let mut run = Self::queued(pipeline);
        let now = Utc::now();
        run.status = RunStatus::Failure;
        run.started_at = Some(now);
        run.completed_at = Some(now);
        run.duration_ms = Some(500);
        if let Some(stage) = run.stages.first_mut() {
            stage.status = StageStatus::Failure;
            stage.started_at = Some(now);
            stage.completed_at = Some(now);
            if let Some(step) = stage.steps.first_mut() {
                step.status = StepStatus::Failure;
                step.started_at = Some(now);
                step.completed_at = Some(now);
                step.exit_code = Some(1);
            }
        }
        run
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_pipeline_fixture() {
        let pipeline = PipelineFixture::simple();
        assert_eq!(pipeline.name, "test-pipeline");
        assert_eq!(pipeline.definition.stages.len(), 1);
    }

    #[test]
    fn test_multi_stage_pipeline_fixture() {
        let pipeline = PipelineFixture::multi_stage();
        assert_eq!(pipeline.definition.stages.len(), 3);
    }

    #[test]
    fn test_parallel_pipeline_fixture() {
        let pipeline = PipelineFixture::parallel();
        assert_eq!(pipeline.definition.stages.len(), 3);
        // Last stage depends on first two
        assert_eq!(pipeline.definition.stages[2].depends_on.len(), 2);
    }

    #[test]
    fn test_run_fixtures() {
        let pipeline = PipelineFixture::simple();

        let queued = RunFixture::queued(&pipeline);
        assert_eq!(queued.status, RunStatus::Queued);

        let running = RunFixture::running(&pipeline);
        assert_eq!(running.status, RunStatus::Running);

        let success = RunFixture::success(&pipeline);
        assert_eq!(success.status, RunStatus::Success);
        assert!(success.completed_at.is_some());

        let failed = RunFixture::failed(&pipeline);
        assert_eq!(failed.status, RunStatus::Failure);
    }
}
