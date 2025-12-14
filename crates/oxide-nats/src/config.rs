//! Configuration for NATS event bus.

use std::time::Duration;

/// Configuration for the NATS event bus.
#[derive(Debug, Clone)]
pub struct NatsConfig {
    /// NATS server URLs (comma-separated for cluster).
    pub urls: Vec<String>,
    /// Stream name for JetStream.
    pub stream_name: String,
    /// Maximum reconnection attempts.
    pub max_reconnect_attempts: Option<usize>,
    /// Reconnection wait time.
    pub reconnect_wait: Duration,
    /// Connection timeout.
    pub connection_timeout: Duration,
    /// Request timeout for JetStream operations.
    pub request_timeout: Duration,
    /// Enable dead letter queue.
    pub enable_dlq: bool,
    /// Dead letter queue stream name.
    pub dlq_stream_name: String,
    /// Maximum delivery attempts before sending to DLQ.
    pub max_deliver: i64,
    /// Message retention period.
    pub max_age: Duration,
}

impl Default for NatsConfig {
    fn default() -> Self {
        Self {
            urls: vec!["nats://localhost:4222".to_string()],
            stream_name: "OXIDE_EVENTS".to_string(),
            max_reconnect_attempts: None, // Unlimited
            reconnect_wait: Duration::from_secs(2),
            connection_timeout: Duration::from_secs(10),
            request_timeout: Duration::from_secs(5),
            enable_dlq: true,
            dlq_stream_name: "OXIDE_DLQ".to_string(),
            max_deliver: 3,
            max_age: Duration::from_secs(86400 * 7), // 7 days
        }
    }
}

impl NatsConfig {
    /// Create a new config with a single URL.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            urls: vec![url.into()],
            ..Default::default()
        }
    }

    /// Set multiple server URLs for cluster support.
    pub fn with_urls(mut self, urls: Vec<String>) -> Self {
        self.urls = urls;
        self
    }

    /// Set the stream name.
    pub fn with_stream_name(mut self, name: impl Into<String>) -> Self {
        self.stream_name = name.into();
        self
    }

    /// Set max reconnection attempts.
    pub fn with_max_reconnects(mut self, max: usize) -> Self {
        self.max_reconnect_attempts = Some(max);
        self
    }

    /// Enable or disable dead letter queue.
    pub fn with_dlq(mut self, enable: bool) -> Self {
        self.enable_dlq = enable;
        self
    }

    /// Set max delivery attempts before DLQ.
    pub fn with_max_deliver(mut self, max: i64) -> Self {
        self.max_deliver = max;
        self
    }
}
