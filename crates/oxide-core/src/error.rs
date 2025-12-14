//! Error types for Oxide CI.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    // Pipeline errors
    #[error("Pipeline not found: {0}")]
    PipelineNotFound(String),

    #[error("Invalid pipeline definition: {0}")]
    InvalidPipeline(String),

    #[error("Pipeline validation failed: {0}")]
    PipelineValidation(String),

    // Run errors
    #[error("Run not found: {0}")]
    RunNotFound(String),

    #[error("Run already completed")]
    RunAlreadyCompleted,

    #[error("Run cancelled: {reason}")]
    RunCancelled { reason: String },

    #[error("Run timeout after {minutes} minutes")]
    RunTimeout { minutes: u32 },

    // Agent errors
    #[error("Agent not found: {0}")]
    AgentNotFound(String),

    #[error("No available agents matching labels: {0:?}")]
    NoAvailableAgents(Vec<String>),

    #[error("Agent disconnected: {0}")]
    AgentDisconnected(String),

    // Step errors
    #[error("Step failed with exit code {exit_code}: {message}")]
    StepFailed { exit_code: i32, message: String },

    #[error("Step timeout after {minutes} minutes")]
    StepTimeout { minutes: u32 },

    // Plugin errors
    #[error("Plugin not found: {0}")]
    PluginNotFound(String),

    #[error("Plugin execution failed: {0}")]
    PluginExecutionFailed(String),

    #[error("Plugin load failed: {0}")]
    PluginLoadFailed(String),

    // Secret errors
    #[error("Secret not found: {0}")]
    SecretNotFound(String),

    #[error("Secret provider not configured: {0}")]
    SecretProviderNotConfigured(String),

    #[error("Secret access denied: {0}")]
    SecretAccessDenied(String),

    // Cache errors
    #[error("Cache miss for key: {0}")]
    CacheMiss(String),

    #[error("Cache upload failed: {0}")]
    CacheUploadFailed(String),

    // Licensing errors
    #[error("License invalid: {0}")]
    LicenseInvalid(String),

    #[error("License expired")]
    LicenseExpired,

    #[error("License suspended: {reason}")]
    LicenseSuspended { reason: String },

    #[error("License quota exceeded: {resource} ({used}/{limit})")]
    LicenseQuotaExceeded {
        resource: String,
        used: u64,
        limit: u64,
    },

    // Billing errors
    #[error("Payment failed: {0}")]
    PaymentFailed(String),

    #[error("Subscription not found: {0}")]
    SubscriptionNotFound(String),

    // Auth errors
    #[error("Authentication required")]
    AuthenticationRequired,

    #[error("Authorization denied: {0}")]
    AuthorizationDenied(String),

    #[error("Invalid token: {0}")]
    InvalidToken(String),

    // Approval errors
    #[error("Approval required for gate: {0}")]
    ApprovalRequired(String),

    #[error("Approval rejected: {0}")]
    ApprovalRejected(String),

    #[error("Approval expired")]
    ApprovalExpired,

    // Infrastructure errors
    #[error("Database error: {0}")]
    Database(String),

    #[error("Event bus error: {0}")]
    EventBus(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),

    // Generic
    #[error("Internal error: {0}")]
    Internal(String),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::Serialization(err.to_string())
    }
}
