//! NATS event bus implementation for Oxide CI.

mod bus;
pub mod config;
pub mod health;
pub mod metrics;

pub use bus::{NatsEventBus, StreamInfo};
pub use config::NatsConfig;
pub use health::{HealthCheck, HealthStatus};
pub use metrics::{MetricsSnapshot, NatsMetrics};
