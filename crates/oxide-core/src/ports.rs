//! Port traits (hexagonal architecture).
//!
//! These traits define the interfaces between the core domain and external adapters.

use crate::agent::Agent;
use crate::cache::{CacheEntry, CacheRestoreRequest, CacheSaveRequest};
use crate::events::Event;
use crate::ids::*;
use crate::pipeline::{Pipeline, PipelineDefinition};
use crate::run::Run;
use crate::secrets::SecretValue;
use crate::{Error, Result};
use async_trait::async_trait;
use std::pin::Pin;
use futures::Stream;

/// Stream of events.
pub type EventStream = Pin<Box<dyn Stream<Item = Result<Event>> + Send>>;

/// Event bus for publishing and subscribing to events.
#[async_trait]
pub trait EventBus: Send + Sync {
    /// Publish an event.
    async fn publish(&self, event: Event) -> Result<()>;

    /// Subscribe to events matching a pattern.
    /// Pattern supports wildcards: `run.*.started`, `agent.>`
    async fn subscribe(&self, pattern: &str) -> Result<EventStream>;
}

/// Repository for pipeline definitions.
#[async_trait]
pub trait PipelineRepository: Send + Sync {
    /// Create a new pipeline.
    async fn create(&self, definition: &PipelineDefinition) -> Result<Pipeline>;

    /// Get a pipeline by ID.
    async fn get(&self, id: PipelineId) -> Result<Option<Pipeline>>;

    /// Get a pipeline by name.
    async fn get_by_name(&self, name: &str) -> Result<Option<Pipeline>>;

    /// List all pipelines.
    async fn list(&self, limit: u32, offset: u32) -> Result<Vec<Pipeline>>;

    /// Update a pipeline.
    async fn update(&self, id: PipelineId, definition: &PipelineDefinition) -> Result<Pipeline>;

    /// Delete a pipeline.
    async fn delete(&self, id: PipelineId) -> Result<()>;
}

/// Repository for pipeline runs.
#[async_trait]
pub trait RunRepository: Send + Sync {
    /// Create a new run.
    async fn create(&self, run: &Run) -> Result<RunId>;

    /// Get a run by ID.
    async fn get(&self, id: RunId) -> Result<Option<Run>>;

    /// Get runs for a pipeline.
    async fn get_by_pipeline(
        &self,
        pipeline_id: PipelineId,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<Run>>;

    /// Get the next run number for a pipeline.
    async fn next_run_number(&self, pipeline_id: PipelineId) -> Result<u32>;

    /// Update a run.
    async fn update(&self, run: &Run) -> Result<()>;

    /// Get queued runs.
    async fn get_queued(&self, limit: u32) -> Result<Vec<Run>>;
}

/// Repository for agents.
#[async_trait]
pub trait AgentRepository: Send + Sync {
    /// Register a new agent.
    async fn register(&self, agent: &Agent) -> Result<AgentId>;

    /// Get an agent by ID.
    async fn get(&self, id: AgentId) -> Result<Option<Agent>>;

    /// List all agents.
    async fn list(&self) -> Result<Vec<Agent>>;

    /// List available agents matching labels.
    async fn list_available(&self, labels: &[String]) -> Result<Vec<Agent>>;

    /// Update agent status.
    async fn update(&self, agent: &Agent) -> Result<()>;

    /// Update agent heartbeat.
    async fn heartbeat(&self, id: AgentId) -> Result<()>;

    /// Deregister an agent.
    async fn deregister(&self, id: AgentId) -> Result<()>;

    /// Get stale agents (no heartbeat within duration).
    async fn get_stale(&self, threshold_seconds: u64) -> Result<Vec<Agent>>;
}

/// Secret provider for retrieving secrets from various backends.
#[async_trait]
pub trait SecretProvider: Send + Sync {
    /// Get a secret value.
    async fn get(&self, path: &str, key: Option<&str>) -> Result<SecretValue>;

    /// Check if provider is healthy.
    async fn health_check(&self) -> Result<()>;
}

/// Cache provider for build caching.
#[async_trait]
pub trait CacheProvider: Send + Sync {
    /// Try to restore a cache entry.
    async fn restore(&self, request: &CacheRestoreRequest) -> Result<Option<CacheEntry>>;

    /// Save a cache entry.
    async fn save(&self, request: &CacheSaveRequest) -> Result<CacheEntry>;

    /// Delete a cache entry.
    async fn delete(&self, key: &str) -> Result<()>;

    /// List cache entries.
    async fn list(&self, prefix: Option<&str>, limit: u32) -> Result<Vec<CacheEntry>>;
}

/// License validator.
#[async_trait]
pub trait LicenseValidator: Send + Sync {
    /// Validate a license key.
    async fn validate(&self, license_key: &str, machine_id: &str) -> Result<LicenseInfo>;

    /// Check if a feature is enabled.
    async fn has_feature(&self, license_key: &str, feature: &str) -> Result<bool>;

    /// Check usage against limits.
    async fn check_quota(&self, license_key: &str, resource: &str, count: u64) -> Result<bool>;
}

/// License information.
#[derive(Debug, Clone)]
pub struct LicenseInfo {
    pub id: String,
    pub policy: String,
    pub status: LicenseStatus,
    pub features: Vec<String>,
    pub limits: std::collections::HashMap<String, u64>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LicenseStatus {
    Active,
    Expired,
    Suspended,
    Invalid,
}

/// Billing service.
#[async_trait]
pub trait BillingService: Send + Sync {
    /// Report usage for metered billing.
    async fn report_usage(&self, subscription_id: &str, quantity: u64) -> Result<()>;

    /// Get subscription status.
    async fn get_subscription(&self, subscription_id: &str) -> Result<SubscriptionInfo>;
}

/// Subscription information.
#[derive(Debug, Clone)]
pub struct SubscriptionInfo {
    pub id: String,
    pub status: String,
    pub plan: String,
    pub current_period_end: chrono::DateTime<chrono::Utc>,
}

/// Plugin host for executing WASM plugins.
#[async_trait]
pub trait PluginHost: Send + Sync {
    /// Load a plugin.
    async fn load(&self, name: &str) -> Result<()>;

    /// Execute a plugin.
    async fn execute(&self, name: &str, input: PluginInput) -> Result<PluginOutput>;

    /// Unload a plugin.
    async fn unload(&self, name: &str) -> Result<()>;
}

/// Input to a plugin.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginInput {
    pub variables: std::collections::HashMap<String, String>,
    pub secrets: std::collections::HashMap<String, String>,
    pub workspace: String,
}

/// Output from a plugin.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginOutput {
    pub success: bool,
    pub exit_code: i32,
    pub outputs: std::collections::HashMap<String, String>,
    pub error: Option<String>,
}

/// Notification sender.
#[async_trait]
pub trait NotificationSender: Send + Sync {
    /// Send a notification.
    async fn send(&self, notification: &Notification) -> Result<()>;
}

/// Notification to send.
#[derive(Debug, Clone)]
pub struct Notification {
    pub channel_type: String,
    pub config: serde_json::Value,
    pub title: String,
    pub body: String,
    pub url: Option<String>,
    pub severity: NotificationSeverity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationSeverity {
    Info,
    Warning,
    Error,
    Critical,
}
