//! Pipeline handlers.

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use oxide_core::ids::PipelineId;
use oxide_core::pipeline::PipelineDefinition;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::state::AppState;

#[derive(Deserialize)]
pub struct ListParams {
    #[serde(default = "default_limit")]
    pub limit: u32,
    #[serde(default)]
    pub offset: u32,
}

fn default_limit() -> u32 {
    20
}

#[derive(Serialize)]
pub struct PipelineResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize)]
pub struct ListPipelinesResponse {
    pub pipelines: Vec<PipelineResponse>,
    pub total: usize,
}

pub async fn list_pipelines(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListParams>,
) -> Result<Json<ListPipelinesResponse>, (StatusCode, String)> {
    let pipelines = state
        .pipelines
        .list(params.limit, params.offset)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let responses: Vec<PipelineResponse> = pipelines
        .iter()
        .map(|p| PipelineResponse {
            id: p.id.to_string(),
            name: p.name.clone(),
            description: p.definition.description.clone(),
            created_at: p.created_at.to_rfc3339(),
            updated_at: p.updated_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(ListPipelinesResponse {
        total: responses.len(),
        pipelines: responses,
    }))
}

pub async fn create_pipeline(
    State(state): State<Arc<AppState>>,
    Json(definition): Json<PipelineDefinition>,
) -> Result<(StatusCode, Json<PipelineResponse>), (StatusCode, String)> {
    let pipeline = state
        .pipelines
        .create(&definition)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(PipelineResponse {
            id: pipeline.id.to_string(),
            name: pipeline.name,
            description: pipeline.definition.description,
            created_at: pipeline.created_at.to_rfc3339(),
            updated_at: pipeline.updated_at.to_rfc3339(),
        }),
    ))
}

pub async fn get_pipeline(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<PipelineResponse>, (StatusCode, String)> {
    let pipeline_id: PipelineId = id
        .parse()
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid pipeline ID".to_string()))?;

    let pipeline = state
        .pipelines
        .get(pipeline_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Pipeline not found".to_string()))?;

    Ok(Json(PipelineResponse {
        id: pipeline.id.to_string(),
        name: pipeline.name,
        description: pipeline.definition.description,
        created_at: pipeline.created_at.to_rfc3339(),
        updated_at: pipeline.updated_at.to_rfc3339(),
    }))
}

pub async fn delete_pipeline(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let pipeline_id: PipelineId = id
        .parse()
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid pipeline ID".to_string()))?;

    state
        .pipelines
        .delete(pipeline_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}
