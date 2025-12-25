//! NATS-based event bus implementation with advanced features.

use async_nats::jetstream::{
    self, consumer::pull::Config as ConsumerConfig, stream::Config as StreamConfig,
};
use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::STANDARD};
use futures::StreamExt;
use oxide_core::events::Event;
use oxide_core::ports::{EventBus, EventStream};
use oxide_core::{Error, Result};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::config::NatsConfig;
use crate::health::HealthCheck;
use crate::metrics::NatsMetrics;

/// NATS-based event bus using JetStream for durability.
#[derive(Clone)]
pub struct NatsEventBus {
    client: async_nats::Client,
    jetstream: jetstream::Context,
    config: NatsConfig,
    metrics: Arc<NatsMetrics>,
    shutdown: Arc<AtomicBool>,
    #[allow(dead_code)]
    consumers: Arc<RwLock<Vec<String>>>,
}

impl NatsEventBus {
    /// Connect to NATS server and initialize JetStream.
    pub async fn connect(url: &str) -> Result<Self> {
        info!("Connecting to NATS at {}", url);

        let client = async_nats::connect(url)
            .await
            .map_err(|e| Error::EventBus(format!("Failed to connect to NATS: {}", e)))?;

        let jetstream = jetstream::new(client.clone());

        let stream_name = "OXIDE_EVENTS".to_string();

        // Ensure stream exists
        let stream_config = jetstream::stream::Config {
            name: stream_name.clone(),
            subjects: vec![
                "run.>".to_string(),
                "stage.>".to_string(),
                "step.>".to_string(),
                "agent.>".to_string(),
                "cache.>".to_string(),
                "secret.>".to_string(),
                "matrix.>".to_string(),
                "approval.>".to_string(),
                "notification.>".to_string(),
                "license.>".to_string(),
                "billing.>".to_string(),
            ],
            retention: jetstream::stream::RetentionPolicy::Limits,
            max_age: Duration::from_secs(86400 * 7), // 7 days
            storage: jetstream::stream::StorageType::File,
            ..Default::default()
        };

        jetstream
            .get_or_create_stream(stream_config)
            .await
            .map_err(|e| Error::EventBus(format!("Failed to create stream: {}", e)))?;

        info!("Connected to NATS and initialized JetStream");

        let config = NatsConfig::new(url);
        let metrics = NatsMetrics::new();
        metrics.set_connected(true);

        Ok(Self {
            client,
            jetstream,
            config,
            metrics,
            shutdown: Arc::new(AtomicBool::new(false)),
            consumers: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// Connect with custom configuration.
    pub async fn connect_with_config(config: NatsConfig) -> Result<Self> {
        let urls = config.urls.join(",");
        info!("Connecting to NATS at {}", urls);

        let metrics = NatsMetrics::new();

        let client = async_nats::ConnectOptions::new()
            .connection_timeout(config.connection_timeout)
            .request_timeout(Some(config.request_timeout))
            .retry_on_initial_connect()
            .connect(&urls)
            .await
            .map_err(|e| Error::EventBus(format!("Failed to connect to NATS: {}", e)))?;

        metrics.set_connected(true);

        let jetstream = jetstream::new(client.clone());

        let stream_config = StreamConfig {
            name: config.stream_name.clone(),
            subjects: vec![
                "run.>".to_string(),
                "stage.>".to_string(),
                "step.>".to_string(),
                "agent.>".to_string(),
                "cache.>".to_string(),
                "secret.>".to_string(),
                "matrix.>".to_string(),
                "approval.>".to_string(),
                "notification.>".to_string(),
                "license.>".to_string(),
                "billing.>".to_string(),
            ],
            retention: jetstream::stream::RetentionPolicy::Limits,
            max_age: config.max_age,
            storage: jetstream::stream::StorageType::File,
            ..Default::default()
        };

        jetstream
            .get_or_create_stream(stream_config)
            .await
            .map_err(|e| Error::EventBus(format!("Failed to create stream: {}", e)))?;

        // Create DLQ stream if enabled
        if config.enable_dlq {
            let dlq_config = StreamConfig {
                name: config.dlq_stream_name.clone(),
                subjects: vec!["dlq.>".to_string()],
                retention: jetstream::stream::RetentionPolicy::Limits,
                max_age: Duration::from_secs(86400 * 30),
                storage: jetstream::stream::StorageType::File,
                ..Default::default()
            };

            jetstream
                .get_or_create_stream(dlq_config)
                .await
                .map_err(|e| Error::EventBus(format!("Failed to create DLQ stream: {}", e)))?;

            info!("Dead letter queue stream initialized");
        }

        info!("Connected to NATS and initialized JetStream");

        Ok(Self {
            client,
            jetstream,
            config,
            metrics,
            shutdown: Arc::new(AtomicBool::new(false)),
            consumers: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// Get the underlying NATS client.
    pub fn client(&self) -> &async_nats::Client {
        &self.client
    }

    /// Get the JetStream context.
    pub fn jetstream(&self) -> &jetstream::Context {
        &self.jetstream
    }

    /// Get metrics.
    pub fn metrics(&self) -> &Arc<NatsMetrics> {
        &self.metrics
    }

    /// Check connection health.
    pub fn health_check(&self) -> HealthCheck {
        let connected = self.client.connection_state() == async_nats::connection::State::Connected;
        HealthCheck::from_metrics(&self.metrics, connected)
    }

    /// Check if connected.
    pub fn is_connected(&self) -> bool {
        self.client.connection_state() == async_nats::connection::State::Connected
    }

    /// Check if shutdown was requested.
    pub fn is_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::SeqCst)
    }

    /// Send a message to the dead letter queue.
    pub async fn send_to_dlq(&self, subject: &str, payload: &[u8], reason: &str) -> Result<()> {
        if !self.config.enable_dlq {
            return Ok(());
        }

        let dlq_subject = format!("dlq.{}", subject);
        let dlq_payload = serde_json::json!({
            "original_subject": subject,
            "payload": STANDARD.encode(payload),
            "reason": reason,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        let payload_bytes =
            serde_json::to_vec(&dlq_payload).map_err(|e| Error::Serialization(e.to_string()))?;

        self.jetstream
            .publish(dlq_subject, payload_bytes.into())
            .await
            .map_err(|e| Error::EventBus(format!("Failed to publish to DLQ: {}", e)))?
            .await
            .map_err(|e| Error::EventBus(format!("Failed to confirm DLQ publish: {}", e)))?;

        self.metrics.record_dlq();
        warn!("Message sent to DLQ: {}", reason);

        Ok(())
    }

    /// Subscribe with a consumer group for load balancing.
    pub async fn subscribe_with_group(
        &self,
        pattern: &str,
        group_name: &str,
    ) -> Result<EventStream> {
        debug!(
            "Subscribing to pattern {} with group {}",
            pattern, group_name
        );

        let consumer = self
            .jetstream
            .create_consumer_on_stream(
                ConsumerConfig {
                    durable_name: Some(group_name.to_string()),
                    filter_subject: pattern.to_string(),
                    max_deliver: self.config.max_deliver,
                    ack_wait: Duration::from_secs(30),
                    ..Default::default()
                },
                &self.config.stream_name,
            )
            .await
            .map_err(|e| Error::EventBus(format!("Failed to create consumer: {}", e)))?;

        self.create_event_stream(consumer).await
    }

    /// Replay messages from a specific sequence number.
    pub async fn replay_from_sequence(
        &self,
        pattern: &str,
        start_sequence: u64,
    ) -> Result<EventStream> {
        debug!(
            "Replaying from sequence {} for pattern {}",
            start_sequence, pattern
        );

        let consumer = self
            .jetstream
            .create_consumer_on_stream(
                ConsumerConfig {
                    filter_subject: pattern.to_string(),
                    deliver_policy: jetstream::consumer::DeliverPolicy::ByStartSequence {
                        start_sequence,
                    },
                    ..Default::default()
                },
                &self.config.stream_name,
            )
            .await
            .map_err(|e| Error::EventBus(format!("Failed to create replay consumer: {}", e)))?;

        self.create_event_stream(consumer).await
    }

    /// Replay messages from a specific time.
    pub async fn replay_from_time(
        &self,
        pattern: &str,
        start_time: chrono::DateTime<chrono::Utc>,
    ) -> Result<EventStream> {
        debug!("Replaying from time {} for pattern {}", start_time, pattern);

        // Convert chrono DateTime to time::OffsetDateTime
        let timestamp = start_time.timestamp();
        let nanos = start_time.timestamp_subsec_nanos();
        let offset_time = time::OffsetDateTime::from_unix_timestamp(timestamp)
            .map_err(|e| Error::EventBus(format!("Invalid timestamp: {}", e)))?
            .replace_nanosecond(nanos)
            .map_err(|e| Error::EventBus(format!("Invalid nanoseconds: {}", e)))?;

        let consumer = self
            .jetstream
            .create_consumer_on_stream(
                ConsumerConfig {
                    filter_subject: pattern.to_string(),
                    deliver_policy: jetstream::consumer::DeliverPolicy::ByStartTime {
                        start_time: offset_time,
                    },
                    ..Default::default()
                },
                &self.config.stream_name,
            )
            .await
            .map_err(|e| {
                Error::EventBus(format!("Failed to create time replay consumer: {}", e))
            })?;

        self.create_event_stream(consumer).await
    }

    /// Graceful shutdown - drain all connections.
    pub async fn shutdown(&self) -> Result<()> {
        info!("Initiating graceful shutdown");
        self.shutdown.store(true, Ordering::SeqCst);

        if let Err(e) = self.client.drain().await {
            error!("Error draining NATS connection: {}", e);
        }

        self.metrics.set_connected(false);
        info!("NATS connection drained");

        Ok(())
    }

    /// Get stream info.
    pub async fn stream_info(&self) -> Result<StreamInfo> {
        let mut stream = self
            .jetstream
            .get_stream(&self.config.stream_name)
            .await
            .map_err(|e| Error::EventBus(format!("Failed to get stream: {}", e)))?;

        let info = stream
            .info()
            .await
            .map_err(|e| Error::EventBus(format!("Failed to get stream info: {}", e)))?;

        Ok(StreamInfo {
            name: info.config.name.clone(),
            messages: info.state.messages,
            bytes: info.state.bytes,
            first_seq: info.state.first_sequence,
            last_seq: info.state.last_sequence,
            consumer_count: info.state.consumer_count,
        })
    }

    async fn create_event_stream(
        &self,
        consumer: jetstream::consumer::Consumer<jetstream::consumer::pull::Config>,
    ) -> Result<EventStream> {
        let messages = consumer
            .messages()
            .await
            .map_err(|e| Error::EventBus(format!("Failed to get messages: {}", e)))?;

        let metrics = self.metrics.clone();
        let shutdown = self.shutdown.clone();

        let stream = messages.map(move |msg_result| {
            if shutdown.load(Ordering::SeqCst) {
                return Err(Error::EventBus("Shutdown in progress".to_string()));
            }

            match msg_result {
                Ok(msg) => {
                    let payload_len = msg.payload.len() as u64;
                    metrics.record_receive(payload_len);

                    // ack() returns a future, but we can't await in map
                    // Drop explicitly as fire-and-forget
                    drop(msg.ack());

                    serde_json::from_slice::<Event>(&msg.payload)
                        .map_err(|e| Error::Serialization(e.to_string()))
                }
                Err(e) => Err(Error::EventBus(format!("Message error: {}", e))),
            }
        });

        Ok(Box::pin(stream))
    }
}

/// Information about a JetStream stream.
#[derive(Debug, Clone)]
pub struct StreamInfo {
    pub name: String,
    pub messages: u64,
    pub bytes: u64,
    pub first_seq: u64,
    pub last_seq: u64,
    pub consumer_count: usize,
}

#[async_trait]
impl EventBus for NatsEventBus {
    async fn publish(&self, event: Event) -> Result<()> {
        if self.is_shutdown() {
            return Err(Error::EventBus(
                "Cannot publish: shutdown in progress".to_string(),
            ));
        }

        let subject = event.subject();
        let payload =
            serde_json::to_vec(&event).map_err(|e| Error::Serialization(e.to_string()))?;

        let payload_len = payload.len() as u64;
        debug!("Publishing event to {}", subject);

        match self
            .jetstream
            .publish(subject.clone(), payload.into())
            .await
        {
            Ok(ack_future) => {
                ack_future
                    .await
                    .map_err(|e| Error::EventBus(format!("Failed to confirm publish: {}", e)))?;
                self.metrics.record_publish(payload_len);
                Ok(())
            }
            Err(e) => {
                self.metrics.record_publish_failure();
                Err(Error::EventBus(format!(
                    "Failed to publish to {}: {}",
                    subject, e
                )))
            }
        }
    }

    async fn subscribe(&self, pattern: &str) -> Result<EventStream> {
        debug!("Subscribing to pattern: {}", pattern);

        let consumer = self
            .jetstream
            .create_consumer_on_stream(
                ConsumerConfig {
                    filter_subject: pattern.to_string(),
                    max_deliver: self.config.max_deliver,
                    ack_wait: Duration::from_secs(30),
                    ..Default::default()
                },
                &self.config.stream_name,
            )
            .await
            .map_err(|e| Error::EventBus(format!("Failed to create consumer: {}", e)))?;

        self.create_event_stream(consumer).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_builder() {
        let config = NatsConfig::new("nats://localhost:4222")
            .with_stream_name("TEST_STREAM")
            .with_max_reconnects(5)
            .with_dlq(true)
            .with_max_deliver(5);

        assert_eq!(config.stream_name, "TEST_STREAM");
        assert_eq!(config.max_reconnect_attempts, Some(5));
        assert!(config.enable_dlq);
        assert_eq!(config.max_deliver, 5);
    }

    #[tokio::test]
    #[ignore] // Requires NATS server
    async fn test_connect() {
        let bus = NatsEventBus::connect("nats://localhost:4222").await;
        assert!(bus.is_ok());
    }

    #[tokio::test]
    #[ignore] // Requires NATS server
    async fn test_health_check() {
        let bus = NatsEventBus::connect("nats://localhost:4222")
            .await
            .unwrap();
        let health = bus.health_check();
        assert!(health.status.is_healthy());
        assert!(health.connected);
    }
}
