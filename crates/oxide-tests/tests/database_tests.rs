//! Database integration tests.
//!
//! Run with: `cargo test -p oxide-tests --test database_tests --features integration`

#![cfg(feature = "integration")]

use oxide_core::ports::{PipelineRepository, RunRepository};
use oxide_db::{PgPipelineRepository, PgRunRepository};
use oxide_tests::{
    context::TestContext,
    fixtures::{PipelineFixture, RunFixture},
};

#[tokio::test]
async fn test_pipeline_crud() {
    let ctx = TestContext::postgres_only()
        .await
        .expect("Failed to create context");

    let repo = PgPipelineRepository::new(ctx.db.pool().clone());

    // Create. The repository assigns the id, so the returned entity — not the
    // fixture — is what every later lookup must use.
    let fixture = PipelineFixture::simple();
    let pipeline = repo
        .create(&fixture.definition)
        .await
        .expect("Failed to create pipeline");

    // Read
    let found = repo
        .get(pipeline.id)
        .await
        .expect("Failed to get pipeline")
        .expect("Pipeline not found");
    assert_eq!(found.name, pipeline.name);

    // List
    let all = repo.list(10, 0).await.expect("Failed to list pipelines");
    assert_eq!(all.len(), 1);

    // Delete
    repo.delete(pipeline.id)
        .await
        .expect("Failed to delete pipeline");
    let gone = repo.get(pipeline.id).await.expect("Failed to get pipeline");
    assert!(gone.is_none());
}

#[tokio::test]
async fn test_pipeline_list_pagination() {
    let ctx = TestContext::postgres_only()
        .await
        .expect("Failed to create context");

    let repo = PgPipelineRepository::new(ctx.db.pool().clone());

    // Create multiple pipelines
    for i in 0..5 {
        let mut fixture = PipelineFixture::simple();
        fixture.definition.name = format!("pipeline-{}", i);
        repo.create(&fixture.definition)
            .await
            .expect("Failed to create pipeline");
    }

    // Test pagination
    let page1 = repo.list(2, 0).await.expect("Failed to list");
    assert_eq!(page1.len(), 2);

    let page2 = repo.list(2, 2).await.expect("Failed to list");
    assert_eq!(page2.len(), 2);

    let page3 = repo.list(2, 4).await.expect("Failed to list");
    assert_eq!(page3.len(), 1);
}

#[tokio::test]
async fn test_run_crud() {
    let ctx = TestContext::postgres_only()
        .await
        .expect("Failed to create context");

    let pipeline_repo = PgPipelineRepository::new(ctx.db.pool().clone());
    let run_repo = PgRunRepository::new(ctx.db.pool().clone());

    // Create pipeline first. The run's foreign key must point at the id the
    // repository assigned, not at the fixture's.
    let fixture = PipelineFixture::simple();
    let pipeline = pipeline_repo
        .create(&fixture.definition)
        .await
        .expect("Failed to create pipeline");

    // Create run
    let run = RunFixture::queued(&pipeline);
    run_repo.create(&run).await.expect("Failed to create run");

    // Read
    let found = run_repo
        .get(run.id)
        .await
        .expect("Failed to get run")
        .expect("Run not found");
    assert_eq!(found.pipeline_id, pipeline.id);

    // List by pipeline
    let runs = run_repo
        .get_by_pipeline(pipeline.id, 10, 0)
        .await
        .expect("Failed to list runs");
    assert_eq!(runs.len(), 1);
}

#[tokio::test]
async fn test_run_status_update() {
    let ctx = TestContext::postgres_only()
        .await
        .expect("Failed to create context");

    let pipeline_repo = PgPipelineRepository::new(ctx.db.pool().clone());
    let run_repo = PgRunRepository::new(ctx.db.pool().clone());

    // Setup
    let fixture = PipelineFixture::simple();
    let pipeline = pipeline_repo.create(&fixture.definition).await.unwrap();

    let mut run = RunFixture::queued(&pipeline);
    run_repo.create(&run).await.unwrap();

    // Update status
    run.status = oxide_core::run::RunStatus::Running;
    run.started_at = Some(chrono::Utc::now());
    run_repo.update(&run).await.expect("Failed to update run");

    // Verify
    let updated = run_repo.get(run.id).await.unwrap().unwrap();
    assert_eq!(updated.status, oxide_core::run::RunStatus::Running);
    assert!(updated.started_at.is_some());
}

#[tokio::test]
async fn test_concurrent_writes() {
    let ctx = TestContext::postgres_only()
        .await
        .expect("Failed to create context");

    let repo = PgPipelineRepository::new(ctx.db.pool().clone());

    // Spawn concurrent writes
    let handles: Vec<_> = (0..10)
        .map(|i| {
            let repo = repo.clone();
            tokio::spawn(async move {
                let mut fixture = PipelineFixture::simple();
                fixture.definition.name = format!("concurrent-{}", i);
                repo.create(&fixture.definition).await
            })
        })
        .collect();

    // Wait for all
    for handle in handles {
        handle.await.unwrap().expect("Concurrent write failed");
    }

    // Verify all created
    let all = repo.list(20, 0).await.unwrap();
    assert_eq!(all.len(), 10);
}

#[tokio::test]
async fn test_run_stages_round_trip() {
    let ctx = TestContext::postgres_only()
        .await
        .expect("Failed to create context");

    let pipeline_repo = PgPipelineRepository::new(ctx.db.pool().clone());
    let run_repo = PgRunRepository::new(ctx.db.pool().clone());

    let fixture = PipelineFixture::multi_stage();
    let pipeline = pipeline_repo
        .create(&fixture.definition)
        .await
        .expect("Failed to create pipeline");

    let run = RunFixture::queued(&pipeline);
    let expected: Vec<String> = run.stages.iter().map(|s| s.name.clone()).collect();
    let expected_steps: Vec<usize> = run.stages.iter().map(|s| s.steps.len()).collect();
    run_repo.create(&run).await.expect("Failed to create run");

    let found = run_repo.get(run.id).await.unwrap().unwrap();

    // Order is part of the contract: stages are a sequence, not a set.
    let names: Vec<String> = found.stages.iter().map(|s| s.name.clone()).collect();
    assert_eq!(
        names, expected,
        "stages must read back in the order written"
    );

    let steps: Vec<usize> = found.stages.iter().map(|s| s.steps.len()).collect();
    assert_eq!(steps, expected_steps, "each stage must keep its own steps");

    for (before, after) in run.stages.iter().zip(found.stages.iter()) {
        assert_eq!(after.id, before.id);
        assert_eq!(after.status, before.status);
        for (b, a) in before.steps.iter().zip(after.steps.iter()) {
            assert_eq!(a.id, b.id);
            assert_eq!(a.name, b.name);
            assert_eq!(a.plugin, b.plugin);
            assert_eq!(a.status, b.status);
        }
    }
}

#[tokio::test]
async fn test_stage_and_step_progress_persists() {
    let ctx = TestContext::postgres_only()
        .await
        .expect("Failed to create context");

    let pipeline_repo = PgPipelineRepository::new(ctx.db.pool().clone());
    let run_repo = PgRunRepository::new(ctx.db.pool().clone());

    let fixture = PipelineFixture::multi_stage();
    let pipeline = pipeline_repo.create(&fixture.definition).await.unwrap();

    let mut run = RunFixture::queued(&pipeline);
    run_repo.create(&run).await.expect("Failed to create run");

    // A run's stages change as it progresses; persisting only the initial
    // shape would be nearly as useless as persisting nothing.
    let started = chrono::Utc::now();
    run.stages[0].status = oxide_core::run::StageStatus::Success;
    run.stages[0].started_at = Some(started);
    run.stages[0].duration_ms = Some(1234);
    run.stages[0].steps[0].status = oxide_core::run::StepStatus::Success;
    run.stages[0].steps[0].exit_code = Some(0);
    run.stages[0].steps[0]
        .outputs
        .insert("artifact".to_string(), "app.tar.gz".to_string());
    run.stages[1].status = oxide_core::run::StageStatus::Running;

    run_repo.update(&run).await.expect("Failed to update run");

    let found = run_repo.get(run.id).await.unwrap().unwrap();
    assert_eq!(
        found.stages.len(),
        run.stages.len(),
        "update must not drop stages"
    );
    assert_eq!(
        found.stages[0].status,
        oxide_core::run::StageStatus::Success
    );
    assert_eq!(found.stages[0].duration_ms, Some(1234));
    assert!(found.stages[0].started_at.is_some());
    assert_eq!(
        found.stages[1].status,
        oxide_core::run::StageStatus::Running
    );

    let step = &found.stages[0].steps[0];
    assert_eq!(step.status, oxide_core::run::StepStatus::Success);
    assert_eq!(step.exit_code, Some(0));
    assert_eq!(
        step.outputs.get("artifact").map(String::as_str),
        Some("app.tar.gz"),
        "step outputs must survive the round trip"
    );
}
