//! Metrics for NATS event bus observability.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// Metrics for the NATS event bus.
#[derive(Debug, Default)]
pub struct NatsMetrics {
    /// Total messages published.
    pub messages_published: AtomicU64,
    /// Total messages received.
    pub messages_received: AtomicU64,
    /// Total publish failures.
    pub publish_failures: AtomicU64,
    /// Total messages sent to DLQ.
    pub messages_dlq: AtomicU64,
    /// Total reconnection attempts.
    pub reconnect_attempts: AtomicU64,
    /// Current connection state (0 = disconnected, 1 = connected).
    pub connected: AtomicU64,
    /// Total bytes published.
    pub bytes_published: AtomicU64,
    /// Total bytes received.
    pub bytes_received: AtomicU64,
}

impl NatsMetrics {
    /// Create new metrics instance.
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Record a successful publish.
    pub fn record_publish(&self, bytes: u64) {
        self.messages_published.fetch_add(1, Ordering::Relaxed);
        self.bytes_published.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Record a publish failure.
    pub fn record_publish_failure(&self) {
        self.publish_failures.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a received message.
    pub fn record_receive(&self, bytes: u64) {
        self.messages_received.fetch_add(1, Ordering::Relaxed);
        self.bytes_received.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Record a message sent to DLQ.
    pub fn record_dlq(&self) {
        self.messages_dlq.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a reconnection attempt.
    pub fn record_reconnect(&self) {
        self.reconnect_attempts.fetch_add(1, Ordering::Relaxed);
    }

    /// Set connection state.
    pub fn set_connected(&self, connected: bool) {
        self.connected.store(connected as u64, Ordering::Relaxed);
    }

    /// Get a snapshot of current metrics.
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            messages_published: self.messages_published.load(Ordering::Relaxed),
            messages_received: self.messages_received.load(Ordering::Relaxed),
            publish_failures: self.publish_failures.load(Ordering::Relaxed),
            messages_dlq: self.messages_dlq.load(Ordering::Relaxed),
            reconnect_attempts: self.reconnect_attempts.load(Ordering::Relaxed),
            connected: self.connected.load(Ordering::Relaxed) == 1,
            bytes_published: self.bytes_published.load(Ordering::Relaxed),
            bytes_received: self.bytes_received.load(Ordering::Relaxed),
        }
    }
}

/// A point-in-time snapshot of metrics.
#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub messages_published: u64,
    pub messages_received: u64,
    pub publish_failures: u64,
    pub messages_dlq: u64,
    pub reconnect_attempts: u64,
    pub connected: bool,
    pub bytes_published: u64,
    pub bytes_received: u64,
}

/// Timer for measuring operation latency.
pub struct LatencyTimer {
    start: Instant,
}

impl LatencyTimer {
    /// Start a new latency timer.
    pub fn start() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    /// Get elapsed duration in milliseconds.
    pub fn elapsed_ms(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }
}
