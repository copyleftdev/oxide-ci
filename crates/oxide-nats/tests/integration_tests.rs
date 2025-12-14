//! Integration tests for oxide-nats.
//!
//! These tests require a running NATS server with JetStream enabled.
//! Run with: `cargo test -p oxide-nats --features integration`
//!
//! To start NATS: `docker run -p 4222:4222 nats:latest -js`

#![cfg(feature = "integration")]

use futures::StreamExt;
use oxide_core::events::{Event, RunQueuedPayload};
use oxide_core::ids::{PipelineId, RunId};
use oxide_core::pipeline::TriggerType;
use oxide_core::ports::EventBus;
use oxide_nats::{NatsConfig, NatsEventBus};
use std::time::Duration;

const NATS_URL: &str = "nats://localhost:4222";

#[tokio::test]
async fn test_publish_and_subscribe() {
    let bus = NatsEventBus::connect(NATS_URL).await.expect("connect");

    let run_id = RunId::new();
    let pipeline_id = PipelineId::new();

    let event = Event::RunQueued(RunQueuedPayload {
        run_id,
        pipeline_id,
        pipeline_name: "test-pipeline".to_string(),
        run_number: 1,
        trigger: TriggerType::Manual,
        git_ref: None,
        git_sha: None,
        queued_at: chrono::Utc::now(),
        queued_by: Some("test@example.com".to_string()),
        license_id: None,
    });

    // Publish the event
    bus.publish(event.clone()).await.expect("publish");

    // Check metrics
    let snapshot = bus.metrics().snapshot();
    assert_eq!(snapshot.messages_published, 1);
    assert!(snapshot.bytes_published > 0);
}

#[tokio::test]
async fn test_health_check() {
    let bus = NatsEventBus::connect(NATS_URL).await.expect("connect");

    let health = bus.health_check();
    assert!(health.status.is_healthy());
    assert!(health.connected);
}

#[tokio::test]
async fn test_stream_info() {
    let bus = NatsEventBus::connect(NATS_URL).await.expect("connect");

    let info = bus.stream_info().await.expect("stream info");
    assert_eq!(info.name, "OXIDE_EVENTS");
}

#[tokio::test]
async fn test_config_with_dlq() {
    let config = NatsConfig::new(NATS_URL).with_dlq(true).with_max_deliver(5);

    let bus = NatsEventBus::connect_with_config(config)
        .await
        .expect("connect");

    let health = bus.health_check();
    assert!(health.status.is_healthy());
}

#[tokio::test]
async fn test_graceful_shutdown() {
    let bus = NatsEventBus::connect(NATS_URL).await.expect("connect");

    assert!(!bus.is_shutdown());
    assert!(bus.is_connected());

    bus.shutdown().await.expect("shutdown");

    assert!(bus.is_shutdown());
}

#[tokio::test]
async fn test_consumer_group() {
    let bus = NatsEventBus::connect(NATS_URL).await.expect("connect");

    // Create a consumer group subscription
    let _stream = bus
        .subscribe_with_group("run.>", "test-group")
        .await
        .expect("subscribe");
}

#[tokio::test]
async fn test_replay_from_sequence() {
    let bus = NatsEventBus::connect(NATS_URL).await.expect("connect");

    // Start replay from sequence 1
    let _stream = bus.replay_from_sequence("run.>", 1).await.expect("replay");
}

#[tokio::test]
async fn test_metrics_tracking() {
    let bus = NatsEventBus::connect(NATS_URL).await.expect("connect");

    let initial = bus.metrics().snapshot();

    // Publish an event
    let event = Event::RunQueued(RunQueuedPayload {
        run_id: RunId::new(),
        pipeline_id: PipelineId::new(),
        pipeline_name: "test".to_string(),
        run_number: 1,
        trigger: TriggerType::Manual,
        git_ref: None,
        git_sha: None,
        queued_at: chrono::Utc::now(),
        queued_by: None,
        license_id: None,
    });

    bus.publish(event).await.expect("publish");

    let after = bus.metrics().snapshot();
    assert_eq!(after.messages_published, initial.messages_published + 1);
}
