//! Run handlers.

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use oxide_core::ids::{PipelineId, RunId};
use oxide_core::pipeline::TriggerType;
use oxide_core::run::{Run, RunStatus, TriggerInfo};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::state::AppState;

#[derive(Deserialize)]
pub struct ListRunsParams {
    #[serde(default = "default_limit")]
    pub limit: u32,
    #[serde(default)]
    pub offset: u32,
}

fn default_limit() -> u32 {
    20
}

#[derive(Serialize)]
pub struct RunResponse {
    pub id: String,
    pub pipeline_id: String,
    pub run_number: u32,
    pub status: String,
    pub git_ref: Option<String>,
    pub git_sha: Option<String>,
    pub queued_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub duration_ms: Option<u64>,
}

impl From<&Run> for RunResponse {
    fn from(run: &Run) -> Self {
        Self {
            id: run.id.to_string(),
            pipeline_id: run.pipeline_id.to_string(),
            run_number: run.run_number,
            status: format!("{:?}", run.status).to_lowercase(),
            git_ref: run.git_ref.clone(),
            git_sha: run.git_sha.clone(),
            queued_at: run.queued_at.to_rfc3339(),
            started_at: run.started_at.map(|t| t.to_rfc3339()),
            completed_at: run.completed_at.map(|t| t.to_rfc3339()),
            duration_ms: run.duration_ms,
        }
    }
}

#[derive(Serialize)]
pub struct ListRunsResponse {
    pub runs: Vec<RunResponse>,
    pub total: usize,
}

#[derive(Deserialize)]
pub struct TriggerRunRequest {
    pub git_ref: Option<String>,
    pub git_sha: Option<String>,
    #[serde(default)]
    pub variables: HashMap<String, String>,
}

pub async fn list_runs(
    State(state): State<Arc<AppState>>,
    Path(pipeline_id): Path<String>,
    Query(params): Query<ListRunsParams>,
) -> Result<Json<ListRunsResponse>, (StatusCode, String)> {
    let pid: PipelineId = pipeline_id
        .parse()
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid pipeline ID".to_string()))?;

    let runs = state
        .runs
        .get_by_pipeline(pid, params.limit, params.offset)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let responses: Vec<RunResponse> = runs.iter().map(RunResponse::from).collect();

    Ok(Json(ListRunsResponse {
        total: responses.len(),
        runs: responses,
    }))
}

pub async fn trigger_run(
    State(state): State<Arc<AppState>>,
    Path(pipeline_id): Path<String>,
    Json(request): Json<TriggerRunRequest>,
) -> Result<(StatusCode, Json<RunResponse>), (StatusCode, String)> {
    let pid: PipelineId = pipeline_id
        .parse()
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid pipeline ID".to_string()))?;

    // Verify pipeline exists
    let pipeline = state
        .pipelines
        .get(pid)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Pipeline not found".to_string()))?;

    // Get next run number
    let run_number = state
        .runs
        .next_run_number(pid)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let now = chrono::Utc::now();
    let run = Run {
        id: RunId::default(),
        pipeline_id: pid,
        pipeline_name: pipeline.name.clone(),
        run_number,
        status: RunStatus::Queued,
        trigger: TriggerInfo {
            trigger_type: TriggerType::Api,
            triggered_by: None,
            source: Some("api".to_string()),
        },
        git_ref: request.git_ref,
        git_sha: request.git_sha,
        variables: request.variables,
        stages: vec![],
        queued_at: now,
        started_at: None,
        completed_at: None,
        duration_ms: None,
        billable_minutes: None,
    };

    let run_id = state
        .runs
        .create(&run)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Publish run queued event
    let event = oxide_core::events::Event::RunQueued(oxide_core::events::RunQueuedPayload {
        run_id,
        pipeline_id: pid,
        pipeline_name: pipeline.name.clone(),
        run_number,
        trigger: TriggerType::Api,
        git_ref: run.git_ref.clone(),
        git_sha: run.git_sha.clone(),
        queued_at: now,
        queued_by: None,
        license_id: None,
    });
    let _ = state.event_bus.publish(event).await;

    Ok((StatusCode::CREATED, Json(RunResponse::from(&run))))
}

pub async fn get_run(
    State(state): State<Arc<AppState>>,
    Path((pipeline_id, run_id)): Path<(String, String)>,
) -> Result<Json<RunResponse>, (StatusCode, String)> {
    let _pid: PipelineId = pipeline_id
        .parse()
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid pipeline ID".to_string()))?;

    let rid: RunId = run_id
        .parse()
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid run ID".to_string()))?;

    let run = state
        .runs
        .get(rid)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Run not found".to_string()))?;

    Ok(Json(RunResponse::from(&run)))
}

pub async fn cancel_run(
    State(state): State<Arc<AppState>>,
    Path((pipeline_id, run_id)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, String)> {
    let _pid: PipelineId = pipeline_id
        .parse()
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid pipeline ID".to_string()))?;

    let rid: RunId = run_id
        .parse()
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid run ID".to_string()))?;

    let mut run = state
        .runs
        .get(rid)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Run not found".to_string()))?;

    run.status = RunStatus::Cancelled;
    run.completed_at = Some(chrono::Utc::now());

    state
        .runs
        .update(&run)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}
