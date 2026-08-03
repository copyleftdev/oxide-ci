//! PostgreSQL implementation of RunRepository.

use async_trait::async_trait;
use oxide_core::ids::{AgentId, StageId, StepId};
use oxide_core::ids::{PipelineId, RunId};
use oxide_core::ports::RunRepository;
use oxide_core::run::{Run, RunStatus, Stage, StageStatus, Step, StepStatus, TriggerInfo};
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

    fn stage_status_to_str(status: &StageStatus) -> &'static str {
        match status {
            StageStatus::Pending => "pending",
            StageStatus::Waiting => "waiting",
            StageStatus::Running => "running",
            StageStatus::Success => "success",
            StageStatus::Failure => "failure",
            StageStatus::Cancelled => "cancelled",
            StageStatus::Skipped => "skipped",
        }
    }

    fn str_to_stage_status(s: &str) -> StageStatus {
        match s {
            "waiting" => StageStatus::Waiting,
            "running" => StageStatus::Running,
            "success" => StageStatus::Success,
            "failure" => StageStatus::Failure,
            "cancelled" => StageStatus::Cancelled,
            "skipped" => StageStatus::Skipped,
            _ => StageStatus::Pending,
        }
    }

    fn step_status_to_str(status: &StepStatus) -> &'static str {
        match status {
            StepStatus::Pending => "pending",
            StepStatus::Running => "running",
            StepStatus::Success => "success",
            StepStatus::Failure => "failure",
            StepStatus::Cancelled => "cancelled",
            StepStatus::Skipped => "skipped",
        }
    }

    fn str_to_step_status(s: &str) -> StepStatus {
        match s {
            "running" => StepStatus::Running,
            "success" => StepStatus::Success,
            "failure" => StepStatus::Failure,
            "cancelled" => StepStatus::Cancelled,
            "skipped" => StepStatus::Skipped,
            _ => StepStatus::Pending,
        }
    }

    /// Write a run's stages and steps.
    ///
    /// Replaces rather than merges: the run in hand is the whole truth about
    /// its stages, and a partial update would leave stages behind that the
    /// caller has removed. The cascade on run_stages takes the steps with it.
    async fn write_stages(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, run: &Run) -> Result<()> {
        sqlx::query("DELETE FROM run_stages WHERE run_id = $1")
            .bind(run.id.as_uuid())
            .execute(&mut **tx)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        for (stage_index, stage) in run.stages.iter().enumerate() {
            let depends_on: Vec<String> = stage
                .depends_on
                .iter()
                .map(|id| id.as_str().to_string())
                .collect();

            sqlx::query(
                "INSERT INTO run_stages (run_id, stage_index, stage_id, name, display_name, \
                 status, depends_on, agent_id, started_at, completed_at, duration_ms) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
            )
            .bind(run.id.as_uuid())
            .bind(stage_index as i32)
            .bind(stage.id.as_str())
            .bind(&stage.name)
            .bind(&stage.display_name)
            .bind(Self::stage_status_to_str(&stage.status))
            .bind(&depends_on)
            .bind(stage.agent_id.map(|id| *id.as_uuid()))
            .bind(stage.started_at)
            .bind(stage.completed_at)
            .bind(stage.duration_ms.map(|d| d as i64))
            .execute(&mut **tx)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

            for (step_index, step) in stage.steps.iter().enumerate() {
                let outputs = serde_json::to_value(&step.outputs)
                    .map_err(|e| Error::Serialization(e.to_string()))?;

                sqlx::query(
                    "INSERT INTO run_steps (run_id, stage_index, step_index, step_id, name, \
                     display_name, status, plugin, exit_code, outputs, started_at, completed_at, \
                     duration_ms) \
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
                )
                .bind(run.id.as_uuid())
                .bind(stage_index as i32)
                .bind(step_index as i32)
                .bind(step.id.as_str())
                .bind(&step.name)
                .bind(&step.display_name)
                .bind(Self::step_status_to_str(&step.status))
                .bind(&step.plugin)
                .bind(step.exit_code)
                .bind(&outputs)
                .bind(step.started_at)
                .bind(step.completed_at)
                .bind(step.duration_ms.map(|d| d as i64))
                .execute(&mut **tx)
                .await
                .map_err(|e| Error::Database(e.to_string()))?;
            }
        }

        Ok(())
    }

    /// Attach stages and steps to runs already loaded.
    ///
    /// Two queries for any number of runs rather than two per run, because
    /// listing a pipeline's history is the common case and N+1 there is felt.
    async fn load_stages(&self, runs: &mut [Run]) -> Result<()> {
        if runs.is_empty() {
            return Ok(());
        }

        let run_ids: Vec<uuid::Uuid> = runs.iter().map(|run| *run.id.as_uuid()).collect();

        let stage_rows = sqlx::query(
            "SELECT run_id, stage_index, stage_id, name, display_name, status, depends_on, \
             agent_id, started_at, completed_at, duration_ms FROM run_stages \
             WHERE run_id = ANY($1) ORDER BY run_id, stage_index",
        )
        .bind(&run_ids)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        let step_rows = sqlx::query(
            "SELECT run_id, stage_index, step_index, step_id, name, display_name, status, \
             plugin, exit_code, outputs, started_at, completed_at, duration_ms FROM run_steps \
             WHERE run_id = ANY($1) ORDER BY run_id, stage_index, step_index",
        )
        .bind(&run_ids)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        // (run, stage) -> steps, so each stage collects its own in one pass.
        let mut steps_by_stage: HashMap<(uuid::Uuid, i32), Vec<Step>> = HashMap::new();
        for row in &step_rows {
            let outputs: HashMap<String, String> = serde_json::from_value(row.get("outputs"))
                .map_err(|e| Error::Serialization(e.to_string()))?;
            let status: String = row.get("status");

            steps_by_stage
                .entry((row.get("run_id"), row.get("stage_index")))
                .or_default()
                .push(Step {
                    id: StepId::new(row.get::<String, _>("step_id")),
                    name: row.get("name"),
                    display_name: row.get("display_name"),
                    status: Self::str_to_step_status(&status),
                    plugin: row.get("plugin"),
                    exit_code: row.get("exit_code"),
                    outputs,
                    started_at: row.get("started_at"),
                    completed_at: row.get("completed_at"),
                    duration_ms: row.get::<Option<i64>, _>("duration_ms").map(|d| d as u64),
                });
        }

        let mut stages_by_run: HashMap<uuid::Uuid, Vec<Stage>> = HashMap::new();
        for row in &stage_rows {
            let run_id: uuid::Uuid = row.get("run_id");
            let stage_index: i32 = row.get("stage_index");
            let status: String = row.get("status");
            let depends_on: Vec<String> = row.get("depends_on");

            stages_by_run.entry(run_id).or_default().push(Stage {
                id: StageId::new(row.get::<String, _>("stage_id")),
                name: row.get("name"),
                display_name: row.get("display_name"),
                status: Self::str_to_stage_status(&status),
                steps: steps_by_stage
                    .remove(&(run_id, stage_index))
                    .unwrap_or_default(),
                depends_on: depends_on.into_iter().map(StageId::new).collect(),
                agent_id: row
                    .get::<Option<uuid::Uuid>, _>("agent_id")
                    .map(AgentId::from_uuid),
                started_at: row.get("started_at"),
                completed_at: row.get("completed_at"),
                duration_ms: row.get::<Option<i64>, _>("duration_ms").map(|d| d as u64),
            });
        }

        for run in runs.iter_mut() {
            run.stages = stages_by_run.remove(run.id.as_uuid()).unwrap_or_default();
        }

        Ok(())
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

        // One transaction: a run that exists without its stages would read
        // back as a run with no work in it, which is exactly the bug this
        // repository had.
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

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
        .execute(&mut *tx)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Self::write_stages(&mut tx, run).await?;

        tx.commit()
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
            Some(r) => {
                let mut runs = vec![self.row_to_run(&r)?];
                self.load_stages(&mut runs).await?;
                Ok(runs.pop())
            }
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

        let mut runs: Vec<Run> = rows
            .iter()
            .map(|r| self.row_to_run(r))
            .collect::<Result<_>>()?;
        self.load_stages(&mut runs).await?;
        Ok(runs)
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

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        sqlx::query(
            "UPDATE runs SET status = $2, trigger = $3, started_at = $4, completed_at = $5, duration_ms = $6, updated_at = NOW() WHERE id = $1"
        )
        .bind(run.id.as_uuid())
        .bind(Self::status_to_str(&run.status))
        .bind(&trigger_json)
        .bind(run.started_at)
        .bind(run.completed_at)
        .bind(run.duration_ms.map(|d| d as i64))
        .execute(&mut *tx)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        // Stage status is the part that actually changes as a run progresses,
        // so an update that ignored it would persist nothing useful.
        Self::write_stages(&mut tx, run).await?;

        tx.commit()
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

        let mut runs: Vec<Run> = rows
            .iter()
            .map(|r| self.row_to_run(r))
            .collect::<Result<_>>()?;
        self.load_stages(&mut runs).await?;
        Ok(runs)
    }
}
