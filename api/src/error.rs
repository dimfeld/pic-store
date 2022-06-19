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

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Auth error: {0}")]
    AuthError(#[from] pic_store_auth::Error),

    #[error("Not found")]
    NotFound,
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        match self {
            Error::Unauthorized => (StatusCode::UNAUTHORIZED, "401 Unauthorized").into_response(),
            Error::NotFound => (StatusCode::NOT_FOUND, "404 Not Found").into_response(),
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

impl From<biscuit_auth::error::Token> for Error {
    fn from(err: biscuit_auth::error::Token) -> Self {
        Error::AuthError(pic_store_auth::Error::DatalogTokenError(err))
    }
}
