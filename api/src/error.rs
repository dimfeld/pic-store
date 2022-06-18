use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use sea_orm::DbErr;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    DbErr(#[from] DbErr),

    #[error(transparent)]
    BiscuitTokenError(#[from] biscuit_auth::error::Token),

    #[error("Unauthorized")]
    Unauthorized,
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        match self {
            Error::Unauthorized => (StatusCode::UNAUTHORIZED, "401 Unauthorized").into_response(),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": {
                        "details": self.to_string()
                    }
                })),
            )
                .into_response(),
        }
    }
}
