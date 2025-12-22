//! Notification channel configuration and types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Notification channel type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelType {
    Slack,
    Discord,
    Teams,
    Email,
    Webhook,
    PagerDuty,
    OpsGenie,
}

/// Notification channel configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationChannel {
    pub id: uuid::Uuid,
    pub channel_type: ChannelType,
    pub name: String,
    pub enabled: bool,
    pub config: ChannelConfig,
    pub triggers: Vec<NotificationTrigger>,
    pub filters: Option<NotificationFilter>,
}

/// Channel-specific configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChannelConfig {
    Slack(SlackConfig),
    Discord(DiscordConfig),
    Teams(TeamsConfig),
    Email(EmailConfig),
    Webhook(WebhookConfig),
    PagerDuty(PagerDutyConfig),
    OpsGenie(OpsGenieConfig),
}

/// Slack webhook configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackConfig {
    pub webhook_url: String,
    pub channel: Option<String>,
    pub username: String,
    pub icon_emoji: String,
    pub thread_replies: bool,
}

impl Default for SlackConfig {
    fn default() -> Self {
        Self {
            webhook_url: String::new(),
            channel: None,
            username: "Oxide CI".to_string(),
            icon_emoji: ":rocket:".to_string(),
            thread_replies: false,
        }
    }
}

/// Discord webhook configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordConfig {
    pub webhook_url: String,
    pub username: String,
    pub avatar_url: Option<String>,
}

impl Default for DiscordConfig {
    fn default() -> Self {
        Self {
            webhook_url: String::new(),
            username: "Oxide CI".to_string(),
            avatar_url: None,
        }
    }
}

/// Microsoft Teams webhook configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamsConfig {
    pub webhook_url: String,
    pub card_style: CardStyle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardStyle {
    Compact,
    Detailed,
}

impl Default for TeamsConfig {
    fn default() -> Self {
        Self {
            webhook_url: String::new(),
            card_style: CardStyle::Compact,
        }
    }
}

/// Email configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailConfig {
    pub recipients: Vec<String>,
    pub cc: Vec<String>,
    pub reply_to: Option<String>,
    pub subject_prefix: String,
}

impl Default for EmailConfig {
    fn default() -> Self {
        Self {
            recipients: vec![],
            cc: vec![],
            reply_to: None,
            subject_prefix: "[Oxide CI]".to_string(),
        }
    }
}

/// Generic webhook configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    pub url: String,
    pub method: HttpMethod,
    pub headers: HashMap<String, String>,
    pub auth: Option<WebhookAuth>,
    pub retry_count: u32,
    pub timeout_seconds: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HttpMethod {
    POST,
    PUT,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookAuth {
    pub auth_type: AuthType,
    pub token_secret: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthType {
    Bearer,
    Basic,
    Hmac,
}

impl Default for WebhookConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            method: HttpMethod::POST,
            headers: HashMap::new(),
            auth: None,
            retry_count: 3,
            timeout_seconds: 30,
        }
    }
}

/// PagerDuty configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PagerDutyConfig {
    pub routing_key: String,
    pub severity: PagerDutySeverity,
    pub dedupe_key_template: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PagerDutySeverity {
    Critical,
    Error,
    Warning,
    Info,
}

impl Default for PagerDutyConfig {
    fn default() -> Self {
        Self {
            routing_key: String::new(),
            severity: PagerDutySeverity::Error,
            dedupe_key_template: None,
        }
    }
}

/// OpsGenie configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpsGenieConfig {
    pub api_key: String,
    pub region: OpsGenieRegion,
    pub priority: OpsGeniePriority,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OpsGenieRegion {
    Us,
    Eu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OpsGeniePriority {
    P1,
    P2,
    P3,
    P4,
    P5,
}

impl Default for OpsGenieConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            region: OpsGenieRegion::Us,
            priority: OpsGeniePriority::P3,
            tags: vec![],
        }
    }
}

/// Notification trigger events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationTrigger {
    RunQueued,
    RunStarted,
    RunCompleted,
    RunFailed,
    RunCancelled,
    RunTimeout,
    StageFailed,
    StepFailed,
    ApprovalRequested,
    ApprovalGranted,
    ApprovalRejected,
    ApprovalExpired,
    LicenseSuspended,
    PaymentFailed,
}

/// Notification filter for selective notifications.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NotificationFilter {
    pub pipelines: Vec<String>,
    pub branches: Vec<String>,
    pub environments: Vec<String>,
    pub only_on_status_change: bool,
}

impl NotificationFilter {
    /// Check if a notification matches this filter.
    pub fn matches(&self, pipeline: Option<&str>, branch: Option<&str>, env: Option<&str>) -> bool {
        let pipeline_match = self.pipelines.is_empty()
            || pipeline
                .map(|p| self.pipelines.iter().any(|f| f == p))
                .unwrap_or(true);

        let branch_match = self.branches.is_empty()
            || branch
                .map(|b| {
                    self.branches.iter().any(|f| {
                        if f.ends_with('*') {
                            b.starts_with(&f[..f.len() - 1])
                        } else {
                            b == f
                        }
                    })
                })
                .unwrap_or(true);

        let env_match = self.environments.is_empty()
            || env
                .map(|e| self.environments.contains(&e.to_string()))
                .unwrap_or(true);

        pipeline_match && branch_match && env_match
    }
}
