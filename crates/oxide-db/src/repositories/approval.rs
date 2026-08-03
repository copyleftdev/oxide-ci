//! PostgreSQL implementation of ApprovalRepository.

use async_trait::async_trait;
use oxide_core::approval::{ApprovalGate, ApprovalStatus};
use oxide_core::ids::{ApprovalGateId, RunId};
use oxide_core::ports::ApprovalRepository;
use oxide_core::{Error, Result};
use sqlx::{PgPool, Row};

pub struct PgApprovalRepository {
    pool: PgPool,
}

impl PgApprovalRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn status_to_str(status: &ApprovalStatus) -> &'static str {
        match status {
            ApprovalStatus::Pending => "pending",
            ApprovalStatus::Approved => "approved",
            ApprovalStatus::Rejected => "rejected",
            ApprovalStatus::Expired => "expired",
            ApprovalStatus::Bypassed => "bypassed",
        }
    }

    /// The gate itself round-trips through JSONB, so the columns stay a
    /// projection for querying rather than a second source of truth.
    fn row_to_gate(row: &sqlx::postgres::PgRow) -> Result<ApprovalGate> {
        let gate: serde_json::Value = row
            .try_get("gate")
            .map_err(|e| Error::Database(format!("approval_gates.gate could not be read: {e}")))?;
        serde_json::from_value(gate)
            .map_err(|e| Error::Serialization(format!("approval gate is not valid JSON: {e}")))
    }
}

#[async_trait]
impl ApprovalRepository for PgApprovalRepository {
    async fn create(&self, gate: &ApprovalGate) -> Result<()> {
        let gate_json = serde_json::to_value(gate).map_err(|e| {
            Error::Serialization(format!("approval gate could not be serialized: {e}"))
        })?;

        sqlx::query(
            "INSERT INTO approval_gates (id, run_id, pipeline_id, stage_name, environment, status, \
             required_approvers, current_approvals, gate, created_at, expires_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(gate.id.as_uuid())
        .bind(gate.run_id.as_uuid())
        .bind(gate.pipeline_id.as_uuid())
        .bind(&gate.stage_name)
        .bind(&gate.environment)
        .bind(Self::status_to_str(&gate.status))
        .bind(gate.required_approvers as i32)
        .bind(gate.current_approvals as i32)
        .bind(&gate_json)
        .bind(gate.created_at)
        .bind(gate.expires_at)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(())
    }

    async fn get(&self, id: ApprovalGateId) -> Result<Option<ApprovalGate>> {
        let row = sqlx::query("SELECT gate FROM approval_gates WHERE id = $1")
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        row.as_ref().map(Self::row_to_gate).transpose()
    }

    async fn update(&self, gate: &ApprovalGate) -> Result<()> {
        let gate_json = serde_json::to_value(gate).map_err(|e| {
            Error::Serialization(format!("approval gate could not be serialized: {e}"))
        })?;

        let result = sqlx::query(
            "UPDATE approval_gates SET status = $2, current_approvals = $3, gate = $4 WHERE id = $1",
        )
        .bind(gate.id.as_uuid())
        .bind(Self::status_to_str(&gate.status))
        .bind(gate.current_approvals as i32)
        .bind(&gate_json)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        // Silently updating nothing is how a caller ends up believing an
        // approval was recorded when it was not.
        if result.rows_affected() == 0 {
            return Err(Error::ApprovalGateNotFound(gate.id.to_string()));
        }

        Ok(())
    }

    async fn list(&self, run_id: Option<RunId>) -> Result<Vec<ApprovalGate>> {
        let rows =
            match run_id {
                Some(run_id) => sqlx::query(
                    "SELECT gate FROM approval_gates WHERE run_id = $1 ORDER BY created_at DESC",
                )
                .bind(run_id.as_uuid())
                .fetch_all(&self.pool)
                .await,
                None => {
                    sqlx::query("SELECT gate FROM approval_gates ORDER BY created_at DESC")
                        .fetch_all(&self.pool)
                        .await
                }
            }
            .map_err(|e| Error::Database(e.to_string()))?;

        rows.iter().map(Self::row_to_gate).collect()
    }
}
