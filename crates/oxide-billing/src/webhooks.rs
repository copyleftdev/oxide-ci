//! Stripe webhook handlers.

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{info, warn};

#[derive(Debug, Error)]
pub enum WebhookError {
    #[error("Invalid signature")]
    InvalidSignature,
    #[error("Unknown event type: {0}")]
    UnknownEvent(String),
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Handler error: {0}")]
    HandlerError(String),
}

/// Stripe webhook event types we handle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StripeEventType {
    #[serde(rename = "customer.subscription.created")]
    SubscriptionCreated,
    #[serde(rename = "customer.subscription.updated")]
    SubscriptionUpdated,
    #[serde(rename = "customer.subscription.deleted")]
    SubscriptionDeleted,
    #[serde(rename = "invoice.paid")]
    InvoicePaid,
    #[serde(rename = "invoice.payment_failed")]
    InvoicePaymentFailed,
    #[serde(rename = "payment_intent.succeeded")]
    PaymentIntentSucceeded,
    #[serde(rename = "payment_intent.payment_failed")]
    PaymentIntentFailed,
    #[serde(other)]
    Unknown,
}

/// Stripe webhook event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripeEvent {
    pub id: String,
    #[serde(rename = "type")]
    pub event_type: StripeEventType,
    pub data: serde_json::Value,
    pub created: i64,
    pub livemode: bool,
}

/// Payment succeeded event data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentSucceededData {
    pub payment_intent_id: String,
    pub customer_id: String,
    pub amount: i64,
    pub currency: String,
    pub invoice_id: Option<String>,
}

/// Payment failed event data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentFailedData {
    pub payment_intent_id: String,
    pub customer_id: String,
    pub amount: i64,
    pub currency: String,
    pub failure_code: Option<String>,
    pub failure_message: Option<String>,
}

/// Subscription event data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionEventData {
    pub subscription_id: String,
    pub customer_id: String,
    pub status: String,
    pub plan_id: Option<String>,
    pub cancel_at_period_end: bool,
}

/// Webhook handler trait.
#[async_trait::async_trait]
pub trait WebhookHandler: Send + Sync {
    async fn on_subscription_created(
        &self,
        data: SubscriptionEventData,
    ) -> Result<(), WebhookError>;
    async fn on_subscription_updated(
        &self,
        data: SubscriptionEventData,
    ) -> Result<(), WebhookError>;
    async fn on_subscription_deleted(
        &self,
        data: SubscriptionEventData,
    ) -> Result<(), WebhookError>;
    async fn on_payment_succeeded(&self, data: PaymentSucceededData) -> Result<(), WebhookError>;
    async fn on_payment_failed(&self, data: PaymentFailedData) -> Result<(), WebhookError>;
}

/// Process a Stripe webhook event.
pub async fn process_webhook<H: WebhookHandler>(
    handler: &H,
    event: StripeEvent,
) -> Result<(), WebhookError> {
    info!(event_id = %event.id, event_type = ?event.event_type, "Processing Stripe webhook");

    match event.event_type {
        StripeEventType::SubscriptionCreated => {
            let data = parse_subscription_data(&event.data)?;
            handler.on_subscription_created(data).await
        }
        StripeEventType::SubscriptionUpdated => {
            let data = parse_subscription_data(&event.data)?;
            handler.on_subscription_updated(data).await
        }
        StripeEventType::SubscriptionDeleted => {
            let data = parse_subscription_data(&event.data)?;
            handler.on_subscription_deleted(data).await
        }
        StripeEventType::InvoicePaid | StripeEventType::PaymentIntentSucceeded => {
            let data = parse_payment_succeeded(&event.data)?;
            handler.on_payment_succeeded(data).await
        }
        StripeEventType::InvoicePaymentFailed | StripeEventType::PaymentIntentFailed => {
            let data = parse_payment_failed(&event.data)?;
            handler.on_payment_failed(data).await
        }
        StripeEventType::Unknown => {
            warn!(event_id = %event.id, "Ignoring unknown event type");
            Ok(())
        }
    }
}

fn parse_subscription_data(
    data: &serde_json::Value,
) -> Result<SubscriptionEventData, WebhookError> {
    let obj = data
        .get("object")
        .ok_or_else(|| WebhookError::ParseError("Missing object".into()))?;

    Ok(SubscriptionEventData {
        subscription_id: obj["id"].as_str().unwrap_or_default().to_string(),
        customer_id: obj["customer"].as_str().unwrap_or_default().to_string(),
        status: obj["status"].as_str().unwrap_or_default().to_string(),
        plan_id: obj["items"]["data"][0]["price"]["id"]
            .as_str()
            .map(|s| s.to_string()),
        cancel_at_period_end: obj["cancel_at_period_end"].as_bool().unwrap_or(false),
    })
}

fn parse_payment_succeeded(data: &serde_json::Value) -> Result<PaymentSucceededData, WebhookError> {
    let obj = data
        .get("object")
        .ok_or_else(|| WebhookError::ParseError("Missing object".into()))?;

    Ok(PaymentSucceededData {
        payment_intent_id: obj["id"].as_str().unwrap_or_default().to_string(),
        customer_id: obj["customer"].as_str().unwrap_or_default().to_string(),
        amount: obj["amount"].as_i64().unwrap_or(0),
        currency: obj["currency"].as_str().unwrap_or("usd").to_string(),
        invoice_id: obj["invoice"].as_str().map(|s| s.to_string()),
    })
}

fn parse_payment_failed(data: &serde_json::Value) -> Result<PaymentFailedData, WebhookError> {
    let obj = data
        .get("object")
        .ok_or_else(|| WebhookError::ParseError("Missing object".into()))?;
    let error = obj.get("last_payment_error");

    Ok(PaymentFailedData {
        payment_intent_id: obj["id"].as_str().unwrap_or_default().to_string(),
        customer_id: obj["customer"].as_str().unwrap_or_default().to_string(),
        amount: obj["amount"].as_i64().unwrap_or(0),
        currency: obj["currency"].as_str().unwrap_or("usd").to_string(),
        failure_code: error
            .and_then(|e| e["code"].as_str())
            .map(|s| s.to_string()),
        failure_message: error
            .and_then(|e| e["message"].as_str())
            .map(|s| s.to_string()),
    })
}

/// Verify Stripe webhook signature.
pub fn verify_signature(
    _payload: &[u8],
    signature: &str,
    secret: &str,
) -> Result<(), WebhookError> {
    // In production, use proper HMAC verification
    // For now, just check that signature header exists
    if signature.is_empty() || secret.is_empty() {
        return Err(WebhookError::InvalidSignature);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_type_serde() {
        let json = r#""customer.subscription.created""#;
        let event_type: StripeEventType = serde_json::from_str(json).unwrap();
        assert_eq!(event_type, StripeEventType::SubscriptionCreated);
    }

    #[test]
    fn test_unknown_event_type() {
        let json = r#""some.unknown.event""#;
        let event_type: StripeEventType = serde_json::from_str(json).unwrap();
        assert_eq!(event_type, StripeEventType::Unknown);
    }
}
