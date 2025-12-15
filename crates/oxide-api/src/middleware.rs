//! HTTP middleware for the API server.

use axum::{
    body::Body,
    http::{Method, Request, header},
    middleware::Next,
    response::Response,
};
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;

/// Create CORS middleware layer.
pub fn cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION, header::ACCEPT])
        .allow_origin(Any)
}

/// Inject request ID into each request.
pub async fn request_id(mut request: Request<Body>, next: Next) -> Response {
    let request_id = Uuid::new_v4().to_string();
    request
        .headers_mut()
        .insert("x-request-id", request_id.parse().unwrap());

    let mut response = next.run(request).await;
    response
        .headers_mut()
        .insert("x-request-id", request_id.parse().unwrap());

    response
}
