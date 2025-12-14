//! Run and execution types.

use crate::ids::{AgentId, PipelineId, RunId, StageId, StepId};
use crate::pipeline::TriggerType;
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Run {
    pub id: RunId,
    pub pipeline_id: PipelineId,
    pub pipeline_name: String,
    pub run_number: u32,
    pub status: RunStatus,
    pub trigger: TriggerInfo,
    pub git_ref: Option<String>,
    pub git_sha: Option<String>,
    pub variables: HashMap<String, String>,
    pub stages: Vec<Stage>,
    pub queued_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<u64>,
    pub billable_minutes: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Queued,
    Running,
    Success,
    Failure,
    Cancelled,
    Timeout,
    Skipped,
}

impl RunStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            RunStatus::Success
                | RunStatus::Failure
                | RunStatus::Cancelled
                | RunStatus::Timeout
                | RunStatus::Skipped
        )
    }

    pub fn is_success(&self) -> bool {
        matches!(self, RunStatus::Success | RunStatus::Skipped)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TriggerInfo {
    pub trigger_type: TriggerType,
    pub triggered_by: Option<String>,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Stage {
    pub id: StageId,
    pub name: String,
    pub display_name: Option<String>,
    pub status: StageStatus,
    pub steps: Vec<Step>,
    pub depends_on: Vec<StageId>,
    pub agent_id: Option<AgentId>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StageStatus {
    Pending,
    Waiting,
    Running,
    Success,
    Failure,
    Cancelled,
    Skipped,
}

impl StageStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            StageStatus::Success
                | StageStatus::Failure
                | StageStatus::Cancelled
                | StageStatus::Skipped
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Step {
    pub id: StepId,
    pub name: String,
    pub display_name: Option<String>,
    pub status: StepStatus,
    pub plugin: Option<String>,
    pub exit_code: Option<i32>,
    pub outputs: HashMap<String, String>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    Pending,
    Running,
    Success,
    Failure,
    Cancelled,
    Skipped,
}

impl StepStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            StepStatus::Success | StepStatus::Failure | StepStatus::Cancelled | StepStatus::Skipped
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StepLog {
    pub step_id: StepId,
    pub stream: LogStream,
    pub line_number: u32,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LogStream {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CancelReason {
    pub reason: CancelReasonType,
    pub cancelled_by: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CancelReasonType {
    UserRequested,
    Timeout,
    LicenseSuspended,
    QuotaExceeded,
    Superseded,
}
