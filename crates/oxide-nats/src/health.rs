//! Health check for NATS event bus.

use crate::metrics::NatsMetrics;
use std::sync::Arc;

/// Health status of the NATS connection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthStatus {
    /// Healthy and connected.
    Healthy,
    /// Degraded but functional.
    Degraded { reason: String },
    /// Unhealthy and not connected.
    Unhealthy { reason: String },
}

impl HealthStatus {
    /// Check if the status is healthy.
    pub fn is_healthy(&self) -> bool {
        matches!(self, HealthStatus::Healthy)
    }

    /// Check if the service is operational (healthy or degraded).
    pub fn is_operational(&self) -> bool {
        matches!(self, HealthStatus::Healthy | HealthStatus::Degraded { .. })
    }
}

/// Health check result with details.
#[derive(Debug, Clone)]
pub struct HealthCheck {
    pub status: HealthStatus,
    pub connected: bool,
    pub reconnect_attempts: u64,
    pub messages_published: u64,
    pub messages_received: u64,
    pub publish_failures: u64,
}

impl HealthCheck {
    /// Create a health check from metrics.
    pub fn from_metrics(metrics: &Arc<NatsMetrics>, connected: bool) -> Self {
        let snapshot = metrics.snapshot();

        let status = if connected {
            if snapshot.publish_failures > 0 {
                HealthStatus::Degraded {
                    reason: format!("{} publish failures recorded", snapshot.publish_failures),
                }
            } else {
                HealthStatus::Healthy
            }
        } else {
            HealthStatus::Unhealthy {
                reason: "Not connected to NATS".to_string(),
            }
        };

        Self {
            status,
            connected,
            reconnect_attempts: snapshot.reconnect_attempts,
            messages_published: snapshot.messages_published,
            messages_received: snapshot.messages_received,
            publish_failures: snapshot.publish_failures,
        }
    }
}
