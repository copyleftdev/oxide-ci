//! Stripe client wrapper.

use serde::{Deserialize, Serialize};
use thiserror::Error;

// Note: async-stripe Client would be used here in production
// For now we define our own wrapper types

#[derive(Debug, Error)]
pub enum StripeError {
    #[error("Stripe API error: {0}")]
    Api(String),
    #[error("Invalid configuration: {0}")]
    Config(String),
    #[error("Customer not found: {0}")]
    CustomerNotFound(String),
    #[error("Subscription not found: {0}")]
    SubscriptionNotFound(String),
}

/// Stripe client configuration.
#[derive(Debug, Clone)]
pub struct StripeConfig {
    pub api_key: String,
    pub webhook_secret: Option<String>,
}

impl StripeConfig {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            webhook_secret: None,
        }
    }

    pub fn with_webhook_secret(mut self, secret: impl Into<String>) -> Self {
        self.webhook_secret = Some(secret.into());
        self
    }
}

/// Subscription status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionStatus {
    Active,
    PastDue,
    Unpaid,
    Canceled,
    Incomplete,
    IncompleteExpired,
    Trialing,
    Paused,
}

/// Plan information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub id: String,
    pub name: String,
    pub amount: i64,
    pub currency: String,
    pub interval: BillingInterval,
    pub metered: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BillingInterval {
    Month,
    Year,
}

/// Subscription information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub id: String,
    pub customer_id: String,
    pub status: SubscriptionStatus,
    pub plan: Plan,
    pub quantity: Option<i64>,
    pub current_period_start: chrono::DateTime<chrono::Utc>,
    pub current_period_end: chrono::DateTime<chrono::Utc>,
    pub cancel_at_period_end: bool,
    pub trial_end: Option<chrono::DateTime<chrono::Utc>>,
}

/// Stripe client wrapper.
pub struct StripeClient {
    config: StripeConfig,
}

impl StripeClient {
    /// Create a new Stripe client.
    pub fn new(config: StripeConfig) -> Self {
        Self { config }
    }

    /// Get the API key.
    pub fn api_key(&self) -> &str {
        &self.config.api_key
    }

    /// Get webhook secret if configured.
    pub fn webhook_secret(&self) -> Option<&str> {
        self.config.webhook_secret.as_deref()
    }
}

/// Customer information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Customer {
    pub id: String,
    pub email: Option<String>,
    pub name: Option<String>,
    pub metadata: std::collections::HashMap<String, String>,
}

/// Invoice information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invoice {
    pub id: String,
    pub customer_id: String,
    pub subscription_id: Option<String>,
    pub amount_due: i64,
    pub amount_paid: i64,
    pub currency: String,
    pub status: InvoiceStatus,
    pub hosted_invoice_url: Option<String>,
    pub pdf_url: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvoiceStatus {
    Draft,
    Open,
    Paid,
    Uncollectible,
    Void,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stripe_config() {
        let config = StripeConfig::new("sk_test_xxx")
            .with_webhook_secret("whsec_xxx");
        assert!(config.webhook_secret.is_some());
    }

    #[test]
    fn test_subscription_status_serde() {
        let status = SubscriptionStatus::Active;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"active\"");
    }
}
