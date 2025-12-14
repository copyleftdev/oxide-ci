//! Event types matching the AsyncAPI specification.

use crate::agent::{AgentStatus, DisconnectReason, SystemMetrics};
use crate::cache::CacheEvictionReason;
use crate::ids::*;
use crate::pipeline::TriggerType;
use crate::run::{CancelReasonType, LogStream, RunStatus, StageStatus, StepStatus};
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// All events in the Oxide CI system.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    // Run lifecycle
    RunQueued(RunQueuedPayload),
    RunStarted(RunStartedPayload),
    RunCompleted(RunCompletedPayload),
    RunCancelled(RunCancelledPayload),

    // Stage lifecycle
    StageStarted(StageStartedPayload),
    StageCompleted(StageCompletedPayload),

    // Step lifecycle
    StepStarted(StepStartedPayload),
    StepOutput(StepOutputPayload),
    StepCompleted(StepCompletedPayload),

    // Agent
    AgentRegistered(AgentRegisteredPayload),
    AgentHeartbeat(AgentHeartbeatPayload),
    AgentDisconnected(AgentDisconnectedPayload),

    // Cache
    CacheHit(CacheHitPayload),
    CacheMiss(CacheMissPayload),
    CacheUploaded(CacheUploadedPayload),
    CacheEvicted(CacheEvictedPayload),

    // Secrets
    SecretAccessed(SecretAccessedPayload),
    SecretRotated(SecretRotatedPayload),

    // Matrix
    MatrixExpanded(MatrixExpandedPayload),
    MatrixJobStarted(MatrixJobStartedPayload),
    MatrixJobCompleted(MatrixJobCompletedPayload),
    MatrixCompleted(MatrixCompletedPayload),

    // Approval
    ApprovalRequested(ApprovalRequestedPayload),
    ApprovalGranted(ApprovalGrantedPayload),
    ApprovalRejected(ApprovalRejectedPayload),
    ApprovalExpired(ApprovalExpiredPayload),

    // Notification
    NotificationSent(NotificationSentPayload),
    NotificationFailed(NotificationFailedPayload),

    // Licensing
    LicenseValidated(LicenseValidatedPayload),
    LicenseExpired(LicenseExpiredPayload),
    LicenseSuspended(LicenseSuspendedPayload),

    // Billing
    SubscriptionCreated(SubscriptionCreatedPayload),
    PaymentSucceeded(PaymentSucceededPayload),
    PaymentFailed(PaymentFailedPayload),
}

impl Event {
    /// Returns the NATS subject for this event.
    pub fn subject(&self) -> String {
        match self {
            Event::RunQueued(p) => format!("run.queued.{}", p.pipeline_id),
            Event::RunStarted(p) => format!("run.started.{}.{}", p.pipeline_id, p.run_id),
            Event::RunCompleted(p) => format!("run.completed.{}.{}", p.pipeline_id, p.run_id),
            Event::RunCancelled(p) => format!("run.cancelled.{}.{}", p.pipeline_id, p.run_id),
            Event::StageStarted(p) => format!("run.{}.stage.{}.started", p.run_id, p.stage_name),
            Event::StageCompleted(p) => {
                format!("run.{}.stage.{}.completed", p.run_id, p.stage_name)
            }
            Event::StepStarted(p) => format!("run.{}.step.{}.started", p.run_id, p.step_id),
            Event::StepOutput(p) => format!("run.{}.step.{}.output", p.run_id, p.step_id),
            Event::StepCompleted(p) => format!("run.{}.step.{}.completed", p.run_id, p.step_id),
            Event::AgentRegistered(_) => "agent.registered".to_string(),
            Event::AgentHeartbeat(p) => format!("agent.{}.heartbeat", p.agent_id),
            Event::AgentDisconnected(p) => format!("agent.{}.disconnected", p.agent_id),
            Event::CacheHit(p) => format!("cache.hit.{}", p.run_id),
            Event::CacheMiss(p) => format!("cache.miss.{}", p.run_id),
            Event::CacheUploaded(p) => format!("cache.uploaded.{}", p.run_id),
            Event::CacheEvicted(p) => format!("cache.evicted.{}", p.cache_id),
            Event::SecretAccessed(p) => format!("secret.accessed.{}", p.secret_id),
            Event::SecretRotated(p) => format!("secret.rotated.{}", p.secret_id),
            Event::MatrixExpanded(p) => format!("matrix.expanded.{}", p.run_id),
            Event::MatrixJobStarted(p) => {
                format!("matrix.{}.job.{}.started", p.matrix_id, p.job_id)
            }
            Event::MatrixJobCompleted(p) => {
                format!("matrix.{}.job.{}.completed", p.matrix_id, p.job_id)
            }
            Event::MatrixCompleted(p) => format!("matrix.{}.completed", p.matrix_id),
            Event::ApprovalRequested(p) => format!("approval.requested.{}", p.gate_id),
            Event::ApprovalGranted(p) => format!("approval.granted.{}", p.gate_id),
            Event::ApprovalRejected(p) => format!("approval.rejected.{}", p.gate_id),
            Event::ApprovalExpired(p) => format!("approval.expired.{}", p.gate_id),
            Event::NotificationSent(p) => format!("notification.sent.{}", p.channel_id),
            Event::NotificationFailed(p) => format!("notification.failed.{}", p.channel_id),
            Event::LicenseValidated(p) => format!("license.validated.{}", p.license_id),
            Event::LicenseExpired(p) => format!("license.expired.{}", p.license_id),
            Event::LicenseSuspended(p) => format!("license.suspended.{}", p.license_id),
            Event::SubscriptionCreated(p) => {
                format!("billing.subscription.created.{}", p.customer_id)
            }
            Event::PaymentSucceeded(p) => format!("billing.payment.succeeded.{}", p.customer_id),
            Event::PaymentFailed(p) => format!("billing.payment.failed.{}", p.customer_id),
        }
    }
}

// === Run Payloads ===

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RunQueuedPayload {
    pub run_id: RunId,
    pub pipeline_id: PipelineId,
    pub pipeline_name: String,
    pub run_number: u32,
    pub trigger: TriggerType,
    pub git_ref: Option<String>,
    pub git_sha: Option<String>,
    pub queued_at: DateTime<Utc>,
    pub queued_by: Option<String>,
    pub license_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RunStartedPayload {
    pub run_id: RunId,
    pub pipeline_id: PipelineId,
    pub pipeline_name: String,
    pub run_number: u32,
    pub agent_id: AgentId,
    pub agent_name: Option<String>,
    pub started_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RunCompletedPayload {
    pub run_id: RunId,
    pub pipeline_id: PipelineId,
    pub pipeline_name: String,
    pub run_number: u32,
    pub status: RunStatus,
    pub duration_ms: u64,
    pub stages_passed: u32,
    pub stages_failed: u32,
    pub completed_at: DateTime<Utc>,
    pub billable_minutes: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RunCancelledPayload {
    pub run_id: RunId,
    pub pipeline_id: PipelineId,
    pub cancelled_by: Option<String>,
    pub reason: CancelReasonType,
    pub cancelled_at: DateTime<Utc>,
}

// === Stage Payloads ===

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StageStartedPayload {
    pub run_id: RunId,
    pub stage_name: String,
    pub stage_index: u32,
    pub step_count: u32,
    pub started_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StageCompletedPayload {
    pub run_id: RunId,
    pub stage_name: String,
    pub stage_index: u32,
    pub status: StageStatus,
    pub duration_ms: u64,
    pub steps_passed: u32,
    pub steps_failed: u32,
    pub completed_at: DateTime<Utc>,
}

// === Step Payloads ===

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StepStartedPayload {
    pub run_id: RunId,
    pub stage_name: String,
    pub step_id: String,
    pub step_name: String,
    pub plugin: Option<String>,
    pub started_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StepOutputPayload {
    pub run_id: RunId,
    pub step_id: String,
    pub stream: LogStream,
    pub line: String,
    pub line_number: u32,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StepCompletedPayload {
    pub run_id: RunId,
    pub stage_name: String,
    pub step_id: String,
    pub step_name: String,
    pub plugin: Option<String>,
    pub status: StepStatus,
    pub exit_code: i32,
    pub duration_ms: u64,
    pub completed_at: DateTime<Utc>,
}

// === Agent Payloads ===

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AgentRegisteredPayload {
    pub agent_id: AgentId,
    pub name: String,
    pub labels: Vec<String>,
    pub version: Option<String>,
    pub registered_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AgentHeartbeatPayload {
    pub agent_id: AgentId,
    pub status: AgentStatus,
    pub current_run_id: Option<RunId>,
    pub system_metrics: Option<SystemMetrics>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AgentDisconnectedPayload {
    pub agent_id: AgentId,
    pub reason: DisconnectReason,
    pub last_heartbeat_at: Option<DateTime<Utc>>,
    pub disconnected_at: DateTime<Utc>,
}

// === Cache Payloads ===

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CacheHitPayload {
    pub run_id: RunId,
    pub step_id: Option<String>,
    pub cache_key: String,
    pub cache_id: CacheEntryId,
    pub size_bytes: u64,
    pub restore_duration_ms: u64,
    pub paths: Vec<String>,
    pub restored_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CacheMissPayload {
    pub run_id: RunId,
    pub step_id: Option<String>,
    pub cache_key: String,
    pub restore_keys_tried: Vec<String>,
    pub will_populate: bool,
    pub missed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CacheUploadedPayload {
    pub run_id: RunId,
    pub step_id: Option<String>,
    pub cache_key: String,
    pub cache_id: CacheEntryId,
    pub size_bytes: u64,
    pub upload_duration_ms: u64,
    pub paths: Vec<String>,
    pub ttl_seconds: Option<u64>,
    pub uploaded_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CacheEvictedPayload {
    pub cache_id: CacheEntryId,
    pub cache_key: String,
    pub reason: CacheEvictionReason,
    pub size_bytes: u64,
    pub age_seconds: u64,
    pub evicted_at: DateTime<Utc>,
}

// === Secret Payloads ===

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SecretAccessedPayload {
    pub secret_id: SecretId,
    pub secret_name: String,
    pub run_id: RunId,
    pub step_id: Option<String>,
    pub pipeline_id: PipelineId,
    pub provider: String,
    pub accessed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SecretRotatedPayload {
    pub secret_id: SecretId,
    pub secret_name: String,
    pub old_version: u32,
    pub new_version: u32,
    pub rotated_by: Option<String>,
    pub rotated_at: DateTime<Utc>,
}

// === Matrix Payloads ===

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MatrixExpandedPayload {
    pub run_id: RunId,
    pub matrix_id: MatrixId,
    pub stage_name: String,
    pub dimensions: HashMap<String, Vec<serde_json::Value>>,
    pub total_combinations: u32,
    pub effective_combinations: u32,
    pub max_parallel: Option<u32>,
    pub fail_fast: bool,
    pub expanded_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MatrixJobStartedPayload {
    pub job_id: JobId,
    pub matrix_id: MatrixId,
    pub run_id: RunId,
    pub stage_name: String,
    pub combination: HashMap<String, serde_json::Value>,
    pub index: u32,
    pub agent_id: AgentId,
    pub started_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MatrixJobCompletedPayload {
    pub job_id: JobId,
    pub matrix_id: MatrixId,
    pub run_id: RunId,
    pub stage_name: String,
    pub combination: HashMap<String, serde_json::Value>,
    pub index: u32,
    pub status: RunStatus,
    pub exit_code: Option<i32>,
    pub duration_ms: u64,
    pub completed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MatrixCompletedPayload {
    pub matrix_id: MatrixId,
    pub run_id: RunId,
    pub stage_name: String,
    pub status: RunStatus,
    pub jobs_succeeded: u32,
    pub jobs_failed: u32,
    pub jobs_cancelled: u32,
    pub jobs_skipped: u32,
    pub total_duration_ms: u64,
    pub cumulative_duration_ms: u64,
    pub completed_at: DateTime<Utc>,
}

// === Approval Payloads ===

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ApprovalRequestedPayload {
    pub gate_id: ApprovalGateId,
    pub run_id: RunId,
    pub pipeline_id: PipelineId,
    pub pipeline_name: String,
    pub stage_name: String,
    pub environment: Option<String>,
    pub triggered_by: Option<String>,
    pub git_ref: Option<String>,
    pub git_sha: Option<String>,
    pub required_approvers: u32,
    pub allowed_approvers: Vec<String>,
    pub message: Option<String>,
    pub approval_url: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub requested_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ApprovalGrantedPayload {
    pub gate_id: ApprovalGateId,
    pub run_id: RunId,
    pub pipeline_id: PipelineId,
    pub stage_name: String,
    pub environment: Option<String>,
    pub approved_by: String,
    pub approver_email: Option<String>,
    pub comment: Option<String>,
    pub current_approvals: u32,
    pub required_approvals: u32,
    pub fully_approved: bool,
    pub approved_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ApprovalRejectedPayload {
    pub gate_id: ApprovalGateId,
    pub run_id: RunId,
    pub pipeline_id: PipelineId,
    pub stage_name: String,
    pub environment: Option<String>,
    pub rejected_by: String,
    pub rejector_email: Option<String>,
    pub reason: Option<String>,
    pub rejected_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ApprovalExpiredPayload {
    pub gate_id: ApprovalGateId,
    pub run_id: RunId,
    pub pipeline_id: PipelineId,
    pub stage_name: String,
    pub environment: Option<String>,
    pub pending_approvals: u32,
    pub timeout_minutes: u32,
    pub expired_at: DateTime<Utc>,
}

// === Notification Payloads ===

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NotificationSentPayload {
    pub notification_id: NotificationChannelId,
    pub channel_id: NotificationChannelId,
    pub channel_type: String,
    pub channel_name: Option<String>,
    pub trigger: String,
    pub run_id: Option<RunId>,
    pub pipeline_id: Option<PipelineId>,
    pub pipeline_name: Option<String>,
    pub status: Option<RunStatus>,
    pub sent_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NotificationFailedPayload {
    pub notification_id: NotificationChannelId,
    pub channel_id: NotificationChannelId,
    pub channel_type: String,
    pub trigger: String,
    pub run_id: Option<RunId>,
    pub error: String,
    pub error_code: Option<String>,
    pub retry_count: u32,
    pub will_retry: bool,
    pub next_retry_at: Option<DateTime<Utc>>,
    pub failed_at: DateTime<Utc>,
}

// === Licensing Payloads ===

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LicenseValidatedPayload {
    pub license_id: String,
    pub policy_name: String,
    pub machine_id: Option<String>,
    pub machine_name: Option<String>,
    pub entitlements: Vec<String>,
    pub usage: Option<LicenseUsage>,
    pub validated_at: DateTime<Utc>,
    pub next_check_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LicenseUsage {
    pub agents_used: u32,
    pub agents_limit: u32,
    pub runs_this_month: u32,
    pub runs_limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LicenseExpiredPayload {
    pub license_id: String,
    pub policy_name: Option<String>,
    pub grace_period_ends_at: Option<DateTime<Utc>>,
    pub expired_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LicenseSuspendedPayload {
    pub license_id: String,
    pub reason: String,
    pub stripe_invoice_id: Option<String>,
    pub suspended_at: DateTime<Utc>,
    pub can_reactivate: bool,
}

// === Billing Payloads ===

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SubscriptionCreatedPayload {
    pub subscription_id: String,
    pub customer_id: String,
    pub status: String,
    pub plan_id: Option<String>,
    pub plan_name: Option<String>,
    pub quantity: Option<u32>,
    pub mrr_cents: Option<u64>,
    pub keygen_license_id: Option<String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PaymentSucceededPayload {
    pub payment_intent_id: String,
    pub invoice_id: Option<String>,
    pub customer_id: String,
    pub subscription_id: Option<String>,
    pub amount: u64,
    pub currency: String,
    pub receipt_url: Option<String>,
    pub paid_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PaymentFailedPayload {
    pub payment_intent_id: String,
    pub invoice_id: Option<String>,
    pub customer_id: String,
    pub subscription_id: Option<String>,
    pub amount: u64,
    pub currency: String,
    pub failure_code: String,
    pub failure_message: Option<String>,
    pub next_retry_at: Option<DateTime<Utc>>,
    pub keygen_license_id: Option<String>,
    pub failed_at: DateTime<Utc>,
}
