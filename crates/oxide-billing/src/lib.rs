//! Stripe billing integration for Oxide CI.
//!
//! Provides metered usage billing, subscription management,
//! and webhook handling for Stripe integration.

pub mod metered;
pub mod stripe;
pub mod webhooks;

pub use metered::{UsageAction, UsageError, UsageRecord, UsageSummary, UsageTracker};
pub use stripe::{
    BillingInterval, Customer, Invoice, InvoiceStatus, Plan, StripeClient, StripeConfig,
    StripeError, Subscription, SubscriptionStatus,
};
pub use webhooks::{
    PaymentFailedData, PaymentSucceededData, StripeEvent, StripeEventType, SubscriptionEventData,
    WebhookError, WebhookHandler, process_webhook, verify_signature,
};
