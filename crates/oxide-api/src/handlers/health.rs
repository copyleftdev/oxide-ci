//! Health check handlers.

use axum::{Json, http::StatusCode};
use serde::Serialize;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

pub async fn ready() -> StatusCode {
    StatusCode::OK
}
