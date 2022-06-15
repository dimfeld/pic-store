use axum::{http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use serde::Serialize;

#[derive(Serialize)]
struct HealthResponse {
    // placeholder until we have real health data
    healthy: bool,
}

async fn health() -> impl IntoResponse {
    (StatusCode::OK, Json(HealthResponse { healthy: true }))
}

pub fn configure() -> Router {
    Router::new().route("/health", get(health))
}
