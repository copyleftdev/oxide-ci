//! Notification sender implementation.

use crate::channels::*;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, info};

#[derive(Debug, Error)]
pub enum NotifyError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Channel not configured: {0}")]
    NotConfigured(String),
    #[error("Delivery failed: {0}")]
    DeliveryFailed(String),
}

/// Notification payload for sending.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPayload {
    pub title: String,
    pub message: String,
    pub status: Option<String>,
    pub pipeline_name: Option<String>,
    pub pipeline_id: Option<String>,
    pub run_id: Option<String>,
    pub run_number: Option<u32>,
    pub branch: Option<String>,
    pub commit_sha: Option<String>,
    pub author: Option<String>,
    pub url: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl NotificationPayload {
    pub fn new(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
            status: None,
            pipeline_name: None,
            pipeline_id: None,
            run_id: None,
            run_number: None,
            branch: None,
            commit_sha: None,
            author: None,
            url: None,
            timestamp: chrono::Utc::now(),
        }
    }
}

/// Trait for notification senders.
#[async_trait]
pub trait NotificationSender: Send + Sync {
    async fn send(&self, payload: &NotificationPayload) -> Result<(), NotifyError>;
}

/// Slack notification sender.
pub struct SlackSender {
    config: SlackConfig,
    client: reqwest::Client,
}

impl SlackSender {
    pub fn new(config: SlackConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    fn build_message(&self, payload: &NotificationPayload) -> serde_json::Value {
        let color = match payload.status.as_deref() {
            Some("success") => "#36a64f",
            Some("failure") | Some("failed") => "#dc3545",
            Some("cancelled") => "#6c757d",
            _ => "#0366d6",
        };

        let mut fields = vec![];
        if let Some(ref pipeline) = payload.pipeline_name {
            fields.push(serde_json::json!({"title": "Pipeline", "value": pipeline, "short": true}));
        }
        if let Some(ref branch) = payload.branch {
            fields.push(serde_json::json!({"title": "Branch", "value": branch, "short": true}));
        }
        if let Some(ref sha) = payload.commit_sha {
            fields.push(serde_json::json!({"title": "Commit", "value": &sha[..7.min(sha.len())], "short": true}));
        }

        serde_json::json!({
            "username": self.config.username,
            "icon_emoji": self.config.icon_emoji,
            "attachments": [{
                "color": color,
                "title": payload.title,
                "text": payload.message,
                "fields": fields,
                "ts": payload.timestamp.timestamp()
            }]
        })
    }
}

#[async_trait]
impl NotificationSender for SlackSender {
    async fn send(&self, payload: &NotificationPayload) -> Result<(), NotifyError> {
        debug!(webhook = %self.config.webhook_url, "Sending Slack notification");

        let message = self.build_message(payload);
        let response = self
            .client
            .post(&self.config.webhook_url)
            .json(&message)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(NotifyError::DeliveryFailed(format!(
                "Slack returned {}: {}",
                status, body
            )));
        }

        info!("Slack notification sent successfully");
        Ok(())
    }
}

/// Discord notification sender.
pub struct DiscordSender {
    config: DiscordConfig,
    client: reqwest::Client,
}

impl DiscordSender {
    pub fn new(config: DiscordConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    fn build_embed(&self, payload: &NotificationPayload) -> serde_json::Value {
        let color = match payload.status.as_deref() {
            Some("success") => 0x36a64f,
            Some("failure") | Some("failed") => 0xdc3545,
            Some("cancelled") => 0x6c757d,
            _ => 0x0366d6,
        };

        let mut fields = vec![];
        if let Some(ref pipeline) = payload.pipeline_name {
            fields.push(serde_json::json!({"name": "Pipeline", "value": pipeline, "inline": true}));
        }
        if let Some(ref branch) = payload.branch {
            fields.push(serde_json::json!({"name": "Branch", "value": branch, "inline": true}));
        }

        serde_json::json!({
            "username": self.config.username,
            "avatar_url": self.config.avatar_url,
            "embeds": [{
                "title": payload.title,
                "description": payload.message,
                "color": color,
                "fields": fields,
                "timestamp": payload.timestamp.to_rfc3339()
            }]
        })
    }
}

#[async_trait]
impl NotificationSender for DiscordSender {
    async fn send(&self, payload: &NotificationPayload) -> Result<(), NotifyError> {
        debug!(webhook = %self.config.webhook_url, "Sending Discord notification");

        let embed = self.build_embed(payload);
        let response = self
            .client
            .post(&self.config.webhook_url)
            .json(&embed)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(NotifyError::DeliveryFailed(format!(
                "Discord returned {}: {}",
                status, body
            )));
        }

        info!("Discord notification sent successfully");
        Ok(())
    }
}

/// Generic webhook sender.
pub struct WebhookSender {
    config: WebhookConfig,
    client: reqwest::Client,
}

impl WebhookSender {
    pub fn new(config: WebhookConfig) -> Self {
        let timeout = config.timeout_seconds;
        Self {
            config,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(timeout as u64))
                .build()
                .unwrap_or_default(),
        }
    }
}

#[async_trait]
impl NotificationSender for WebhookSender {
    async fn send(&self, payload: &NotificationPayload) -> Result<(), NotifyError> {
        debug!(url = %self.config.url, "Sending webhook notification");

        let mut request = match self.config.method {
            HttpMethod::POST => self.client.post(&self.config.url),
            HttpMethod::PUT => self.client.put(&self.config.url),
        };

        for (key, value) in &self.config.headers {
            request = request.header(key, value);
        }

        if let Some(ref auth) = self.config.auth {
            match auth.auth_type {
                AuthType::Bearer => {
                    request = request.bearer_auth(&auth.token_secret);
                }
                AuthType::Basic => {
                    request = request.basic_auth(&auth.token_secret, None::<&str>);
                }
                AuthType::Hmac => {
                    // HMAC would require signing the payload
                }
            }
        }

        let response = request.json(payload).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(NotifyError::DeliveryFailed(format!(
                "Webhook returned {}: {}",
                status, body
            )));
        }

        info!("Webhook notification sent successfully");
        Ok(())
    }
}

/// Create a sender for a channel configuration.
pub fn create_sender(config: &ChannelConfig) -> Box<dyn NotificationSender> {
    match config {
        ChannelConfig::Slack(c) => Box::new(SlackSender::new(c.clone())),
        ChannelConfig::Discord(c) => Box::new(DiscordSender::new(c.clone())),
        ChannelConfig::Webhook(c) => Box::new(WebhookSender::new(c.clone())),
        ChannelConfig::Teams(c) => Box::new(WebhookSender::new(WebhookConfig {
            url: c.webhook_url.clone(),
            ..Default::default()
        })),
        ChannelConfig::Email(_) => Box::new(WebhookSender::new(WebhookConfig::default())),
        ChannelConfig::PagerDuty(c) => Box::new(WebhookSender::new(WebhookConfig {
            url: "https://events.pagerduty.com/v2/enqueue".to_string(),
            headers: [("X-Routing-Key".to_string(), c.routing_key.clone())].into(),
            ..Default::default()
        })),
        ChannelConfig::OpsGenie(c) => {
            let base_url = match c.region {
                OpsGenieRegion::Us => "https://api.opsgenie.com",
                OpsGenieRegion::Eu => "https://api.eu.opsgenie.com",
            };
            Box::new(WebhookSender::new(WebhookConfig {
                url: format!("{}/v2/alerts", base_url),
                auth: Some(WebhookAuth {
                    auth_type: AuthType::Bearer,
                    token_secret: c.api_key.clone(),
                }),
                ..Default::default()
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_payload() {
        let payload = NotificationPayload::new("Build Failed", "Pipeline 'main' failed");
        assert_eq!(payload.title, "Build Failed");
    }

    #[test]
    fn test_slack_message_color() {
        let config = SlackConfig::default();
        let sender = SlackSender::new(config);

        let mut payload = NotificationPayload::new("Test", "Message");
        payload.status = Some("success".to_string());
        let msg = sender.build_message(&payload);

        assert!(msg["attachments"][0]["color"].as_str().unwrap().contains("36a64f"));
    }
}
