//! API integration tests.
//!
//! Run with: `cargo test -p oxide-tests --test api_tests --features integration`

#![cfg(feature = "integration")]

use oxide_tests::{
    context::TestContext,
    fixtures::{PipelineFixture, RunFixture},
    helpers::{ApiTestClient, start_test_server},
};
use reqwest::StatusCode;

#[tokio::test]
async fn test_health_endpoint() {
    let ctx = TestContext::new()
        .await
        .expect("Failed to create test context");
    let (addr, _handle) = start_test_server(ctx.db.clone(), ctx.event_bus.clone())
        .await
        .expect("Failed to start server");

    let client = ApiTestClient::new(addr);
    let healthy = client.health().await.expect("Health check failed");
    assert!(healthy);
}

#[tokio::test]
async fn test_list_pipelines_empty() {
    let ctx = TestContext::new()
        .await
        .expect("Failed to create test context");
    let (addr, _handle) = start_test_server(ctx.db.clone(), ctx.event_bus.clone())
        .await
        .expect("Failed to start server");

    let client = ApiTestClient::new(addr);
    let resp = client
        .get("/api/v1/pipelines")
        .await
        .expect("Request failed");

    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = resp.json().await.expect("Failed to parse JSON");
    // The endpoint returns ListPipelinesResponse: { pipelines: [...], total }.
    assert_eq!(body["total"], 0);
    assert!(
        body["pipelines"]
            .as_array()
            .expect("pipelines array")
            .is_empty()
    );
}

#[tokio::test]
async fn test_create_pipeline() {
    let ctx = TestContext::new()
        .await
        .expect("Failed to create test context");
    let (addr, _handle) = start_test_server(ctx.db.clone(), ctx.event_bus.clone())
        .await
        .expect("Failed to start server");

    let client = ApiTestClient::new(addr);
    let pipeline = PipelineFixture::simple();

    let resp = client
        .post("/api/v1/pipelines", &pipeline.definition)
        .await
        .expect("Request failed");

    assert_eq!(resp.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn test_get_pipeline_not_found() {
    let ctx = TestContext::new()
        .await
        .expect("Failed to create test context");
    let (addr, _handle) = start_test_server(ctx.db.clone(), ctx.event_bus.clone())
        .await
        .expect("Failed to start server");

    let client = ApiTestClient::new(addr);
    let resp = client
        .get("/api/v1/pipelines/00000000-0000-0000-0000-000000000000")
        .await
        .expect("Request failed");

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_list_runs_empty() {
    let ctx = TestContext::new()
        .await
        .expect("Failed to create test context");
    let (addr, _handle) = start_test_server(ctx.db.clone(), ctx.event_bus.clone())
        .await
        .expect("Failed to start server");

    let client = ApiTestClient::new(addr);
    // Runs are nested under their pipeline — there is no top-level
    // /api/v1/runs collection in the router.
    let pipeline_id = uuid::Uuid::new_v4();
    let resp = client
        .get(&format!("/api/v1/pipelines/{pipeline_id}/runs"))
        .await
        .expect("Request failed");

    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = resp.json().await.expect("Failed to parse JSON");
    assert_eq!(body["total"], 0);
}

#[tokio::test]
async fn test_list_agents_empty() {
    let ctx = TestContext::new()
        .await
        .expect("Failed to create test context");
    let (addr, _handle) = start_test_server(ctx.db.clone(), ctx.event_bus.clone())
        .await
        .expect("Failed to start server");

    let client = ApiTestClient::new(addr);
    let resp = client.get("/api/v1/agents").await.expect("Request failed");

    assert_eq!(resp.status(), StatusCode::OK);
}
