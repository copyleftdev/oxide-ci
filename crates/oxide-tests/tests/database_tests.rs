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

    // Create
    let pipeline = PipelineFixture::simple();
    repo.create(&pipeline.definition)
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
        let mut pipeline = PipelineFixture::simple();
        pipeline.name = format!("pipeline-{}", i);
        repo.create(&pipeline.definition)
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

    // Create pipeline first
    let pipeline = PipelineFixture::simple();
    pipeline_repo
        .create(&pipeline.definition)
        .await
        .expect("Failed to create pipeline");

    // Create run
    let run = RunFixture::pending(&pipeline);
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
    let pipeline = PipelineFixture::simple();
    pipeline_repo.create(&pipeline.definition).await.unwrap();

    let mut run = RunFixture::pending(&pipeline);
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
                let mut pipeline = PipelineFixture::simple();
                pipeline.name = format!("concurrent-{}", i);
                repo.create(&pipeline).await
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
