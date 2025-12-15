//! Approval gate and environment protection types.

use crate::ids::{ApprovalGateId, PipelineId, RunId};
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Approval gate status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Rejected,
    Expired,
    Bypassed,
}

/// Individual approver action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ApproverAction {
    Approved,
    Rejected,
}

/// An individual approver's response.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Approver {
    pub user_id: String,
    pub user_name: Option<String>,
    pub user_email: Option<String>,
    pub action: ApproverAction,
    pub comment: Option<String>,
    pub acted_at: DateTime<Utc>,
}

/// Approval gate for manual approval workflows.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ApprovalGate {
    pub id: ApprovalGateId,
    pub run_id: RunId,
    pub pipeline_id: PipelineId,
    pub stage_name: String,
    pub environment: Option<String>,
    pub status: ApprovalStatus,
    pub required_approvers: u32,
    pub current_approvals: u32,
    pub approvers: Vec<Approver>,
    pub allowed_approvers: Vec<String>,
    pub prevent_self_approval: bool,
    pub timeout_minutes: u32,
    pub message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

impl ApprovalGate {
    /// Check if the gate is fully approved.
    pub fn is_fully_approved(&self) -> bool {
        self.current_approvals >= self.required_approvers
    }

    /// Check if the gate has expired.
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Check if a user is allowed to approve.
    pub fn can_approve(&self, user_id: &str, triggered_by: Option<&str>) -> bool {
        // Check if user is in allowed approvers
        let is_allowed = self.allowed_approvers.is_empty()
            || self.allowed_approvers.iter().any(|a| {
                a == user_id || a.starts_with("team:") // Team matching would need resolution
            });

        // Check self-approval prevention
        let is_self = triggered_by.is_some_and(|t| t == user_id);
        let self_approval_ok = !self.prevent_self_approval || !is_self;

        // Check if already acted
        let already_acted = self.approvers.iter().any(|a| a.user_id == user_id);

        is_allowed && self_approval_ok && !already_acted
    }

    /// Record an approval.
    pub fn approve(&mut self, approver: Approver) {
        self.approvers.push(approver);
        self.current_approvals += 1;
        if self.is_fully_approved() {
            self.status = ApprovalStatus::Approved;
        }
    }

    /// Record a rejection.
    pub fn reject(&mut self, approver: Approver) {
        self.approvers.push(approver);
        self.status = ApprovalStatus::Rejected;
    }

    /// Mark as expired.
    pub fn expire(&mut self) {
        self.status = ApprovalStatus::Expired;
    }
}

/// Environment protection rule.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EnvironmentProtectionRule {
    pub id: uuid::Uuid,
    pub environment: String,
    pub require_approval: bool,
    pub required_approvers: u32,
    pub allowed_approvers: Vec<String>,
    pub prevent_self_approval: bool,
    pub allowed_branches: Vec<String>,
    pub wait_timer_minutes: Option<u32>,
    pub custom_rules: Vec<CustomProtectionRule>,
}

impl Default for EnvironmentProtectionRule {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            environment: String::new(),
            require_approval: true,
            required_approvers: 1,
            allowed_approvers: vec![],
            prevent_self_approval: true,
            allowed_branches: vec![],
            wait_timer_minutes: None,
            custom_rules: vec![],
        }
    }
}

impl EnvironmentProtectionRule {
    /// Check if a branch is allowed to deploy.
    pub fn is_branch_allowed(&self, branch: &str) -> bool {
        if self.allowed_branches.is_empty() {
            return true;
        }
        self.allowed_branches.iter().any(|pattern| {
            if pattern.ends_with('*') {
                branch.starts_with(&pattern[..pattern.len() - 1])
            } else {
                branch == pattern
            }
        })
    }
}

/// Custom protection rule type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CustomRuleType {
    Webhook,
    StatusCheck,
    TimeWindow,
}

/// Custom protection rule.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CustomProtectionRule {
    pub rule_type: CustomRuleType,
    pub webhook_url: Option<String>,
    pub required_status_checks: Vec<String>,
    pub allowed_time_windows: Vec<TimeWindow>,
}

/// Day of week.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DayOfWeek {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

/// Time window for deployments.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TimeWindow {
    pub days: Vec<DayOfWeek>,
    pub start_time: String,
    pub end_time: String,
    pub timezone: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_approval_gate_approve() {
        let mut gate = ApprovalGate {
            id: ApprovalGateId::new(),
            run_id: RunId::new(),
            pipeline_id: PipelineId::new(),
            stage_name: "deploy".to_string(),
            environment: Some("production".to_string()),
            status: ApprovalStatus::Pending,
            required_approvers: 2,
            current_approvals: 0,
            approvers: vec![],
            allowed_approvers: vec![],
            prevent_self_approval: true,
            timeout_minutes: 60,
            message: Some("Approve deployment?".to_string()),
            created_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::hours(1),
        };

        // First approval
        gate.approve(Approver {
            user_id: "user1".to_string(),
            user_name: Some("User 1".to_string()),
            user_email: None,
            action: ApproverAction::Approved,
            comment: None,
            acted_at: Utc::now(),
        });

        assert_eq!(gate.current_approvals, 1);
        assert_eq!(gate.status, ApprovalStatus::Pending);

        // Second approval - should complete
        gate.approve(Approver {
            user_id: "user2".to_string(),
            user_name: Some("User 2".to_string()),
            user_email: None,
            action: ApproverAction::Approved,
            comment: Some("LGTM".to_string()),
            acted_at: Utc::now(),
        });

        assert_eq!(gate.current_approvals, 2);
        assert_eq!(gate.status, ApprovalStatus::Approved);
        assert!(gate.is_fully_approved());
    }

    #[test]
    fn test_approval_gate_reject() {
        let mut gate = ApprovalGate {
            id: ApprovalGateId::new(),
            run_id: RunId::new(),
            pipeline_id: PipelineId::new(),
            stage_name: "deploy".to_string(),
            environment: None,
            status: ApprovalStatus::Pending,
            required_approvers: 1,
            current_approvals: 0,
            approvers: vec![],
            allowed_approvers: vec![],
            prevent_self_approval: false,
            timeout_minutes: 60,
            message: None,
            created_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::hours(1),
        };

        gate.reject(Approver {
            user_id: "user1".to_string(),
            user_name: None,
            user_email: None,
            action: ApproverAction::Rejected,
            comment: Some("Not ready".to_string()),
            acted_at: Utc::now(),
        });

        assert_eq!(gate.status, ApprovalStatus::Rejected);
    }

    #[test]
    fn test_branch_allowed() {
        let rule = EnvironmentProtectionRule {
            allowed_branches: vec!["main".to_string(), "release/*".to_string()],
            ..Default::default()
        };

        assert!(rule.is_branch_allowed("main"));
        assert!(rule.is_branch_allowed("release/v1.0"));
        assert!(!rule.is_branch_allowed("develop"));
    }

    #[test]
    fn test_can_approve_self_approval() {
        let gate = ApprovalGate {
            id: ApprovalGateId::new(),
            run_id: RunId::new(),
            pipeline_id: PipelineId::new(),
            stage_name: "deploy".to_string(),
            environment: None,
            status: ApprovalStatus::Pending,
            required_approvers: 1,
            current_approvals: 0,
            approvers: vec![],
            allowed_approvers: vec![],
            prevent_self_approval: true,
            timeout_minutes: 60,
            message: None,
            created_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::hours(1),
        };

        // User who triggered cannot self-approve
        assert!(!gate.can_approve("user1", Some("user1")));
        // Different user can approve
        assert!(gate.can_approve("user2", Some("user1")));
    }
}
