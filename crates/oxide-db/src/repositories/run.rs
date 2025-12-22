//! PostgreSQL implementation of RunRepository.

use async_trait::async_trait;
use oxide_core::ids::{PipelineId, RunId};
use oxide_core::ports::RunRepository;
use oxide_core::run::{Run, RunStatus, TriggerInfo};
use oxide_core::{Error, Result};
use sqlx::{PgPool, Row};
use std::collections::HashMap;

/// PostgreSQL implementation of RunRepository.
pub struct PgRunRepository {
    pool: PgPool,
}

impl PgRunRepository {
    /// Create a new PgRunRepository.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn status_to_str(status: &RunStatus) -> &'static str {
        match status {
            RunStatus::Queued => "queued",
            RunStatus::Running => "running",
            RunStatus::Success => "success",
            RunStatus::Failure => "failure",
            RunStatus::Cancelled => "cancelled",
            RunStatus::Timeout => "timeout",
            RunStatus::Skipped => "skipped",
        }
    }

    fn str_to_status(s: &str) -> RunStatus {
        match s {
            "queued" => RunStatus::Queued,
            "running" => RunStatus::Running,
            "success" => RunStatus::Success,
            "failure" => RunStatus::Failure,
            "cancelled" => RunStatus::Cancelled,
            "timeout" => RunStatus::Timeout,
            "skipped" => RunStatus::Skipped,
            _ => RunStatus::Queued,
        }
    }

    fn row_to_run(&self, r: &sqlx::postgres::PgRow) -> Result<Run> {
        let trigger: TriggerInfo = serde_json::from_value(r.get("trigger"))
            .map_err(|e| Error::Serialization(e.to_string()))?;
        let status_str: String = r.get("status");

        Ok(Run {
            id: RunId::from_uuid(r.get::<uuid::Uuid, _>("id")),
            pipeline_id: PipelineId::from_uuid(r.get::<uuid::Uuid, _>("pipeline_id")),
            pipeline_name: String::new(),
            run_number: r.get::<i32, _>("run_number") as u32,
            status: Self::str_to_status(&status_str),
            trigger,
            git_ref: r.get("git_ref"),
            git_sha: r.get("git_sha"),
            variables: HashMap::new(),
            stages: vec![],
            queued_at: r.get("queued_at"),
            started_at: r.get("started_at"),
            completed_at: r.get("completed_at"),
            duration_ms: r.get::<Option<i64>, _>("duration_ms").map(|d| d as u64),
            billable_minutes: None,
        })
    }
}

#[async_trait]
impl RunRepository for PgRunRepository {
    async fn create(&self, run: &Run) -> Result<RunId> {
        let trigger_json =
            serde_json::to_value(&run.trigger).map_err(|e| Error::Serialization(e.to_string()))?;

        sqlx::query(
            r#"INSERT INTO runs (id, pipeline_id, run_number, status, trigger, git_ref, git_sha, queued_at, started_at, completed_at, duration_ms)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)"#
        )
        .bind(run.id.as_uuid())
        .bind(run.pipeline_id.as_uuid())
        .bind(run.run_number as i32)
        .bind(Self::status_to_str(&run.status))
        .bind(&trigger_json)
        .bind(&run.git_ref)
        .bind(&run.git_sha)
        .bind(run.queued_at)
        .bind(run.started_at)
        .bind(run.completed_at)
        .bind(run.duration_ms.map(|d| d as i64))
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(run.id)
    }

    async fn get(&self, id: RunId) -> Result<Option<Run>> {
        let row = sqlx::query(
            "SELECT id, pipeline_id, run_number, status, trigger, git_ref, git_sha, queued_at, started_at, completed_at, duration_ms FROM runs WHERE id = $1"
        )
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        match row {
            Some(r) => Ok(Some(self.row_to_run(&r)?)),
            None => Ok(None),
        }
    }

    async fn get_by_pipeline(
        &self,
        pipeline_id: PipelineId,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<Run>> {
        let rows = sqlx::query(
            "SELECT id, pipeline_id, run_number, status, trigger, git_ref, git_sha, queued_at, started_at, completed_at, duration_ms FROM runs WHERE pipeline_id = $1 ORDER BY run_number DESC LIMIT $2 OFFSET $3"
        )
        .bind(pipeline_id.as_uuid())
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        rows.iter().map(|r| self.row_to_run(r)).collect()
    }

    async fn next_run_number(&self, pipeline_id: PipelineId) -> Result<u32> {
        let row = sqlx::query("SELECT COALESCE(MAX(run_number), 0) + 1 as next_number FROM runs WHERE pipeline_id = $1")
            .bind(pipeline_id.as_uuid())
            .fetch_one(&self.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(row.get::<i32, _>("next_number") as u32)
    }

    async fn update(&self, run: &Run) -> Result<()> {
        let trigger_json =
            serde_json::to_value(&run.trigger).map_err(|e| Error::Serialization(e.to_string()))?;

        sqlx::query(
            "UPDATE runs SET status = $2, trigger = $3, started_at = $4, completed_at = $5, duration_ms = $6, updated_at = NOW() WHERE id = $1"
        )
        .bind(run.id.as_uuid())
        .bind(Self::status_to_str(&run.status))
        .bind(&trigger_json)
        .bind(run.started_at)
        .bind(run.completed_at)
        .bind(run.duration_ms.map(|d| d as i64))
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(())
    }

    async fn get_queued(&self, limit: u32) -> Result<Vec<Run>> {
        let rows = sqlx::query(
            "SELECT id, pipeline_id, run_number, status, trigger, git_ref, git_sha, queued_at, started_at, completed_at, duration_ms FROM runs WHERE status = 'queued' ORDER BY queued_at ASC LIMIT $1"
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        rows.iter().map(|r| self.row_to_run(r)).collect()
    }
}
