use axum::{http::StatusCode, response::IntoResponse, routing::get, Extension, Json, Router};
use sea_orm::{ConnectionTrait, DatabaseConnection, Statement};
use serde::Serialize;

#[derive(Serialize)]
struct HealthResponse {
    /// If the database connection is ok
    database: bool,
    /// If all the other fields indicate healthy status.
    healthy: bool,
}

async fn health(Extension(ref db): Extension<DatabaseConnection>) -> impl IntoResponse {
    let db_result = db
        .query_one(Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            "SELECT 1".to_owned(),
        ))
        .await;

    (
        StatusCode::OK,
        Json(HealthResponse {
            healthy: db_result.is_ok(),
            database: db_result.is_ok(),
        }),
    )
}

pub fn configure() -> Router {
    Router::new().route("/health", get(health))
}
