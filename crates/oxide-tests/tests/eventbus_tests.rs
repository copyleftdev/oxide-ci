//! Event bus integration tests.
//!
//! Run with: `cargo test -p oxide-tests --test eventbus_tests --features integration`

#![cfg(feature = "integration")]

use futures::StreamExt;
use oxide_core::events::{Event, RunQueuedPayload, RunStartedPayload};
use oxide_core::ids::{AgentId, PipelineId, RunId};
use oxide_core::pipeline::TriggerType;
use oxide_core::ports::EventBus;
use oxide_tests::context::TestContext;
use std::time::Duration;

#[tokio::test]
async fn test_publish_event() {
    let ctx = TestContext::nats_only()
        .await
        .expect("Failed to create context");

    let event = Event::RunQueued(RunQueuedPayload {
        run_id: RunId::new(),
        pipeline_id: PipelineId::new(),
        pipeline_name: "test-pipeline".to_string(),
        run_number: 1,
        trigger: TriggerType::Manual,
        git_ref: None,
        git_sha: None,
        queued_at: chrono::Utc::now(),
        queued_by: Some("test@example.com".to_string()),
        license_id: None,
    });

    ctx.event_bus
        .publish(event)
        .await
        .expect("Failed to publish event");

    let metrics = ctx.event_bus.metrics().snapshot();
    assert_eq!(metrics.messages_published, 1);
}

#[tokio::test]
async fn test_subscribe_and_receive() {
    let ctx = TestContext::nats_only()
        .await
        .expect("Failed to create context");

    let run_id = RunId::new();
    let pipeline_id = PipelineId::new();

    // Subscribe to run events
    let mut stream = ctx
        .event_bus
        .subscribe("run.>")
        .await
        .expect("Failed to subscribe");

    // Publish event
    let event = Event::RunQueued(RunQueuedPayload {
        run_id,
        pipeline_id,
        pipeline_name: "test".to_string(),
        run_number: 1,
        trigger: TriggerType::Manual,
        git_ref: None,
        git_sha: None,
        queued_at: chrono::Utc::now(),
        queued_by: None,
        license_id: None,
    });

    ctx.event_bus
        .publish(event)
        .await
        .expect("Failed to publish");

    // Receive with timeout
    let received = tokio::time::timeout(Duration::from_secs(5), stream.next())
        .await
        .expect("Timeout waiting for event")
        .expect("Stream ended");

    match received {
        Ok(Event::RunQueued(payload)) => {
            assert_eq!(payload.run_id, run_id);
            assert_eq!(payload.pipeline_id, pipeline_id);
        }
        _ => panic!("Unexpected event type"),
    }
}

#[tokio::test]
async fn test_consumer_group() {
    let ctx = TestContext::nats_only()
        .await
        .expect("Failed to create context");

    // Create consumer group
    let _stream = ctx
        .event_bus
        .subscribe_with_group("run.>", "test-workers")
        .await
        .expect("Failed to create consumer group");

    // Verify health
    let health = ctx.event_bus.health_check();
    assert!(health.status.is_healthy());
}

#[tokio::test]
async fn test_multiple_event_types() {
    let ctx = TestContext::nats_only()
        .await
        .expect("Failed to create context");

    let run_id = RunId::new();
    let pipeline_id = PipelineId::new();

    // Publish different event types
    let queued = Event::RunQueued(RunQueuedPayload {
        run_id,
        pipeline_id,
        pipeline_name: "test".to_string(),
        run_number: 1,
        trigger: TriggerType::Manual,
        git_ref: None,
        git_sha: None,
        queued_at: chrono::Utc::now(),
        queued_by: None,
        license_id: None,
    });

    let started = Event::RunStarted(RunStartedPayload {
        run_id,
        pipeline_id,
        pipeline_name: "test".to_string(),
        run_number: 1,
        agent_id: AgentId::new(),
        agent_name: Some("test-agent".to_string()),
        started_at: chrono::Utc::now(),
    });

    ctx.event_bus
        .publish(queued)
        .await
        .expect("Failed to publish");
    ctx.event_bus
        .publish(started)
        .await
        .expect("Failed to publish");

    let metrics = ctx.event_bus.metrics().snapshot();
    assert_eq!(metrics.messages_published, 2);
}

#[tokio::test]
async fn test_stream_info() {
    let ctx = TestContext::nats_only()
        .await
        .expect("Failed to create context");

    let info = ctx
        .event_bus
        .stream_info()
        .await
        .expect("Failed to get stream info");

    assert_eq!(info.name, "OXIDE_EVENTS");
}

#[tokio::test]
async fn test_health_check() {
    let ctx = TestContext::nats_only()
        .await
        .expect("Failed to create context");

    let health = ctx.event_bus.health_check();
    assert!(health.status.is_healthy());
    assert!(health.connected);
}

#[tokio::test]
async fn test_graceful_shutdown() {
    let ctx = TestContext::nats_only()
        .await
        .expect("Failed to create context");

    assert!(!ctx.event_bus.is_shutdown());
    ctx.event_bus.shutdown().await.expect("Failed to shutdown");
    assert!(ctx.event_bus.is_shutdown());
}
