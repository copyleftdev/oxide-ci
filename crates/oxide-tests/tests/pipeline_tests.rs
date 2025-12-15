//! End-to-end pipeline integration tests.
//!
//! Run with: `cargo test -p oxide-tests --test pipeline_tests --features integration`

#![cfg(feature = "integration")]

use oxide_core::events::{Event, RunQueuedPayload};
use oxide_core::pipeline::TriggerType;
use oxide_core::ports::{EventBus, PipelineRepository, RunRepository};
use oxide_db::{PgPipelineRepository, PgRunRepository};
use oxide_tests::{
    context::TestContext,
    fixtures::{PipelineFixture, RunFixture},
    helpers::wait_for,
};
use std::time::Duration;

#[tokio::test]
async fn test_pipeline_creation_and_run_queuing() {
    let ctx = TestContext::new().await.expect("Failed to create context");

    let pipeline_repo = PgPipelineRepository::new(ctx.db.pool().clone());
    let run_repo = PgRunRepository::new(ctx.db.pool().clone());

    // Create pipeline
    let pipeline = PipelineFixture::simple();
    pipeline_repo
        .create(&pipeline)
        .await
        .expect("Failed to create pipeline");

    // Create and queue a run
    let run = RunFixture::queued(&pipeline);
    run_repo.create(&run).await.expect("Failed to create run");

    // Publish run queued event
    let event = Event::RunQueued(RunQueuedPayload {
        run_id: run.id,
        pipeline_id: pipeline.id,
        pipeline_name: pipeline.name.clone(),
        run_number: run.run_number,
        trigger: TriggerType::Manual,
        git_ref: run.git_ref.clone(),
        git_sha: run.git_sha.clone(),
        queued_at: run.queued_at,
        queued_by: run.trigger.triggered_by.clone(),
        license_id: None,
    });

    ctx.event_bus
        .publish(event)
        .await
        .expect("Failed to publish event");

    // Verify run exists in database
    let found_run = run_repo
        .get(run.id)
        .await
        .expect("Failed to get run")
        .expect("Run not found");

    assert_eq!(found_run.pipeline_id, pipeline.id);
    assert_eq!(found_run.run_number, 1);
}

#[tokio::test]
async fn test_multi_stage_pipeline_run() {
    let ctx = TestContext::new().await.expect("Failed to create context");

    let pipeline_repo = PgPipelineRepository::new(ctx.db.pool().clone());
    let run_repo = PgRunRepository::new(ctx.db.pool().clone());

    // Create multi-stage pipeline
    let pipeline = PipelineFixture::multi_stage();
    pipeline_repo
        .create(&pipeline)
        .await
        .expect("Failed to create pipeline");

    // Create run with all stages
    let run = RunFixture::queued(&pipeline);
    run_repo.create(&run).await.expect("Failed to create run");

    // Verify stages are created
    let found_run = run_repo.get(run.id).await.unwrap().unwrap();
    assert_eq!(found_run.stages.len(), 3);
    assert_eq!(found_run.stages[0].name, "build");
    assert_eq!(found_run.stages[1].name, "test");
    assert_eq!(found_run.stages[2].name, "deploy");
}

#[tokio::test]
async fn test_parallel_pipeline_dag() {
    let ctx = TestContext::new().await.expect("Failed to create context");

    let pipeline_repo = PgPipelineRepository::new(ctx.db.pool().clone());

    // Create parallel pipeline
    let pipeline = PipelineFixture::parallel();
    pipeline_repo
        .create(&pipeline)
        .await
        .expect("Failed to create pipeline");

    // Verify DAG structure
    let found = pipeline_repo.get(pipeline.id).await.unwrap().unwrap();
    
    // lint and test have no dependencies (can run in parallel)
    assert!(found.definition.stages[0].depends_on.is_empty());
    assert!(found.definition.stages[1].depends_on.is_empty());
    
    // deploy depends on both lint and test
    assert_eq!(found.definition.stages[2].depends_on.len(), 2);
}

#[tokio::test]
async fn test_run_lifecycle() {
    let ctx = TestContext::new().await.expect("Failed to create context");

    let pipeline_repo = PgPipelineRepository::new(ctx.db.pool().clone());
    let run_repo = PgRunRepository::new(ctx.db.pool().clone());

    // Setup
    let pipeline = PipelineFixture::simple();
    pipeline_repo.create(&pipeline).await.unwrap();

    // Create queued run
    let mut run = RunFixture::queued(&pipeline);
    run_repo.create(&run).await.unwrap();

    // Transition to running
    run.status = oxide_core::run::RunStatus::Running;
    run.started_at = Some(chrono::Utc::now());
    run_repo.update(&run).await.unwrap();

    let updated = run_repo.get(run.id).await.unwrap().unwrap();
    assert_eq!(updated.status, oxide_core::run::RunStatus::Running);

    // Transition to success
    run.status = oxide_core::run::RunStatus::Success;
    run.completed_at = Some(chrono::Utc::now());
    run_repo.update(&run).await.unwrap();

    let final_run = run_repo.get(run.id).await.unwrap().unwrap();
    assert_eq!(final_run.status, oxide_core::run::RunStatus::Success);
    assert!(final_run.completed_at.is_some());
}

#[tokio::test]
async fn test_multiple_runs_for_pipeline() {
    let ctx = TestContext::new().await.expect("Failed to create context");

    let pipeline_repo = PgPipelineRepository::new(ctx.db.pool().clone());
    let run_repo = PgRunRepository::new(ctx.db.pool().clone());

    // Create pipeline
    let pipeline = PipelineFixture::simple();
    pipeline_repo.create(&pipeline).await.unwrap();

    // Create multiple runs
    for i in 1..=5 {
        let mut run = RunFixture::queued(&pipeline);
        run.run_number = i;
        run_repo.create(&run).await.unwrap();
    }

    // List runs for pipeline
    let runs = run_repo
        .list_by_pipeline(pipeline.id, 10, 0)
        .await
        .unwrap();

    assert_eq!(runs.len(), 5);
}
