//! Notification channels for Oxide CI.
//!
//! Provides notification delivery to various channels including
//! Slack, Discord, Teams, email, webhooks, PagerDuty, and OpsGenie.

pub mod channels;
pub mod sender;

pub use channels::{
    AuthType, CardStyle, ChannelConfig, ChannelType, DiscordConfig, EmailConfig, HttpMethod,
    NotificationChannel, NotificationFilter, NotificationTrigger, OpsGenieConfig,
    OpsGeniePriority, OpsGenieRegion, PagerDutyConfig, PagerDutySeverity, SlackConfig,
    TeamsConfig, WebhookAuth, WebhookConfig,
};
pub use sender::{
    DiscordSender, NotificationPayload, NotificationSender, NotifyError, SlackSender,
    WebhookSender, create_sender,
};
