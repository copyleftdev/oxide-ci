//! Metered usage reporting for build minutes.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::info;

#[derive(Debug, Error)]
pub enum UsageError {
    #[error("Failed to report usage: {0}")]
    ReportFailed(String),
    #[error("Invalid subscription: {0}")]
    InvalidSubscription(String),
}

/// Usage record for metered billing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageRecord {
    pub subscription_item_id: String,
    pub quantity: i64,
    pub timestamp: DateTime<Utc>,
    pub action: UsageAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UsageAction {
    Increment,
    Set,
}

impl UsageRecord {
    /// Create a new usage record for build minutes.
    pub fn build_minutes(subscription_item_id: impl Into<String>, minutes: i64) -> Self {
        Self {
            subscription_item_id: subscription_item_id.into(),
            quantity: minutes,
            timestamp: Utc::now(),
            action: UsageAction::Increment,
        }
    }

    /// Create a new usage record for agent seats.
    pub fn agent_seats(subscription_item_id: impl Into<String>, count: i64) -> Self {
        Self {
            subscription_item_id: subscription_item_id.into(),
            quantity: count,
            timestamp: Utc::now(),
            action: UsageAction::Set,
        }
    }
}

/// Usage aggregation for reporting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageSummary {
    pub subscription_id: String,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub build_minutes: i64,
    pub storage_gb: f64,
    pub agent_count: i64,
    pub run_count: i64,
}

/// Metered usage tracker.
pub struct UsageTracker {
    subscription_item_id: String,
    pending_minutes: i64,
}

impl UsageTracker {
    pub fn new(subscription_item_id: impl Into<String>) -> Self {
        Self {
            subscription_item_id: subscription_item_id.into(),
            pending_minutes: 0,
        }
    }

    /// Record build minutes for a run.
    pub fn record_build_minutes(&mut self, minutes: i64) {
        self.pending_minutes += minutes;
        info!(minutes = minutes, total = self.pending_minutes, "Recorded build minutes");
    }

    /// Get pending minutes to report.
    pub fn pending_minutes(&self) -> i64 {
        self.pending_minutes
    }

    /// Create a usage record and reset pending.
    pub fn flush(&mut self) -> Option<UsageRecord> {
        if self.pending_minutes > 0 {
            let record = UsageRecord::build_minutes(&self.subscription_item_id, self.pending_minutes);
            self.pending_minutes = 0;
            Some(record)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usage_record_build_minutes() {
        let record = UsageRecord::build_minutes("si_xxx", 10);
        assert_eq!(record.quantity, 10);
        assert_eq!(record.action, UsageAction::Increment);
    }

    #[test]
    fn test_usage_tracker() {
        let mut tracker = UsageTracker::new("si_xxx");
        tracker.record_build_minutes(5);
        tracker.record_build_minutes(3);
        assert_eq!(tracker.pending_minutes(), 8);

        let record = tracker.flush().unwrap();
        assert_eq!(record.quantity, 8);
        assert_eq!(tracker.pending_minutes(), 0);
    }
}
