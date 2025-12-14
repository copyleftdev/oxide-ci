//! NATS-based event bus implementation.

use async_nats::jetstream;
use async_trait::async_trait;
use futures::StreamExt;
use oxide_core::events::Event;
use oxide_core::ports::{EventBus, EventStream};
use oxide_core::{Error, Result};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info};

/// NATS-based event bus using JetStream for durability.
pub struct NatsEventBus {
    client: async_nats::Client,
    jetstream: jetstream::Context,
    stream_name: String,
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

        Ok(Self {
            client,
            jetstream,
            stream_name,
        })
    }

    /// Get the underlying NATS client.
    pub fn client(&self) -> &async_nats::Client {
        &self.client
    }
}

#[async_trait]
impl EventBus for NatsEventBus {
    async fn publish(&self, event: Event) -> Result<()> {
        let subject = event.subject();
        let payload = serde_json::to_vec(&event)
            .map_err(|e| Error::Serialization(e.to_string()))?;

        debug!("Publishing event to {}", subject);

        self.jetstream
            .publish(subject.clone(), payload.into())
            .await
            .map_err(|e| Error::EventBus(format!("Failed to publish to {}: {}", subject, e)))?
            .await
            .map_err(|e| Error::EventBus(format!("Failed to confirm publish: {}", e)))?;

        Ok(())
    }

    async fn subscribe(&self, pattern: &str) -> Result<EventStream> {
        debug!("Subscribing to pattern: {}", pattern);

        let consumer_name = format!("oxide-{}", uuid::Uuid::new_v4());

        let consumer = self
            .jetstream
            .create_consumer_on_stream(
                jetstream::consumer::pull::Config {
                    name: Some(consumer_name),
                    filter_subject: pattern.to_string(),
                    ..Default::default()
                },
                &self.stream_name,
            )
            .await
            .map_err(|e| Error::EventBus(format!("Failed to create consumer: {}", e)))?;

        let messages = consumer
            .messages()
            .await
            .map_err(|e| Error::EventBus(format!("Failed to get messages: {}", e)))?;

        let stream = messages.map(|msg_result| {
            match msg_result {
                Ok(msg) => {
                    // Acknowledge the message
                    if let Err(e) = msg.ack() {
                        error!("Failed to ack message: {}", e);
                    }

                    // Parse the event
                    serde_json::from_slice::<Event>(&msg.payload)
                        .map_err(|e| Error::Serialization(e.to_string()))
                }
                Err(e) => Err(Error::EventBus(format!("Message error: {}", e))),
            }
        });

        Ok(Box::pin(stream))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires NATS server
    async fn test_connect() {
        let bus = NatsEventBus::connect("nats://localhost:4222").await;
        assert!(bus.is_ok());
    }
}
