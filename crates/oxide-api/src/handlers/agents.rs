//! Agent handlers.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use oxide_core::agent::Agent;
use oxide_core::ids::AgentId;
use serde::Serialize;
use std::sync::Arc;

use crate::state::AppState;

#[derive(Serialize)]
pub struct AgentResponse {
    pub id: String,
    pub name: String,
    pub labels: Vec<String>,
    pub version: Option<String>,
    pub os: String,
    pub arch: String,
    pub status: String,
    pub max_concurrent_jobs: u32,
    pub registered_at: String,
    pub last_heartbeat_at: Option<String>,
}

impl From<&Agent> for AgentResponse {
    fn from(agent: &Agent) -> Self {
        Self {
            id: agent.id.to_string(),
            name: agent.name.clone(),
            labels: agent.labels.clone(),
            version: agent.version.clone(),
            os: format!("{:?}", agent.os).to_lowercase(),
            arch: format!("{:?}", agent.arch).to_lowercase(),
            status: format!("{:?}", agent.status).to_lowercase(),
            max_concurrent_jobs: agent.max_concurrent_jobs,
            registered_at: agent.registered_at.to_rfc3339(),
            last_heartbeat_at: agent.last_heartbeat_at.map(|t| t.to_rfc3339()),
        }
    }
}

#[derive(Serialize)]
pub struct ListAgentsResponse {
    pub agents: Vec<AgentResponse>,
    pub total: usize,
}

pub async fn list_agents(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ListAgentsResponse>, (StatusCode, String)> {
    let agents = state
        .agents
        .list()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let responses: Vec<AgentResponse> = agents.iter().map(AgentResponse::from).collect();

    Ok(Json(ListAgentsResponse {
        total: responses.len(),
        agents: responses,
    }))
}

pub async fn get_agent(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<AgentResponse>, (StatusCode, String)> {
    let agent_id: AgentId = id
        .parse()
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid agent ID".to_string()))?;

    let agent = state
        .agents
        .get(agent_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Agent not found".to_string()))?;

    Ok(Json(AgentResponse::from(&agent)))
}

pub async fn deregister_agent(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let agent_id: AgentId = id
        .parse()
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid agent ID".to_string()))?;

    state
        .agents
        .deregister(agent_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}
