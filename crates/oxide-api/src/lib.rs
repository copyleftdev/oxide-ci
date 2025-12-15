//! HTTP/WebSocket API server for Oxide CI.

pub mod handlers;
pub mod middleware;
pub mod routes;
pub mod state;
pub mod ws;

use axum::{Router, middleware as axum_middleware};
use std::sync::Arc;
use tower_http::trace::TraceLayer;

pub use routes::create_router;
pub use state::AppState;

/// Build the complete application router with all middleware.
pub fn build_app(state: Arc<AppState>) -> Router {
    create_router(state)
        .layer(TraceLayer::new_for_http())
        .layer(middleware::cors_layer())
        .layer(axum_middleware::from_fn(middleware::request_id))
}

/// Server configuration.
#[derive(Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8080,
        }
    }
}

impl ServerConfig {
    pub fn addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
