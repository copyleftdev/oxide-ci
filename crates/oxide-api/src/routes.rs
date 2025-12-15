//! API route definitions.

use axum::{
    Router,
    routing::{get, post},
};
use std::sync::Arc;

use crate::handlers::{agents, approvals, health, pipelines, runs};
use crate::state::AppState;

/// Create the main API router.
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .nest("/api/v1", api_routes())
        .route("/health", get(health::health))
        .route("/ready", get(health::ready))
        .with_state(state)
}

fn api_routes() -> Router<Arc<AppState>> {
    Router::new()
        .nest("/pipelines", pipeline_routes())
        .nest("/agents", agent_routes())
        .nest("/approvals", approval_routes())
}

fn approval_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(approvals::list_approvals))
        .route("/{gate_id}", get(approvals::get_approval))
        .route("/{gate_id}/respond", post(approvals::respond_to_approval))
        .route("/{gate_id}/bypass", post(approvals::bypass_approval))
}

fn pipeline_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/",
            get(pipelines::list_pipelines).post(pipelines::create_pipeline),
        )
        .route(
            "/{id}",
            get(pipelines::get_pipeline).delete(pipelines::delete_pipeline),
        )
        .route("/{id}/runs", get(runs::list_runs).post(runs::trigger_run))
        .route("/{pipeline_id}/runs/{run_id}", get(runs::get_run))
        .route(
            "/{pipeline_id}/runs/{run_id}/cancel",
            post(runs::cancel_run),
        )
}

fn agent_routes() -> Router<Arc<AppState>> {
    Router::new().route("/", get(agents::list_agents)).route(
        "/{id}",
        get(agents::get_agent).delete(agents::deregister_agent),
    )
}
