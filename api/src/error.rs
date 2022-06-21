use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use sea_orm::DbErr;
use serde_json::json;
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

impl Error {
    pub fn response_tuple(&self) -> (StatusCode, serde_json::Value) {
        match self {
            Error::Unauthorized => (StatusCode::UNAUTHORIZED, json!("401 Unauthorized")),
            Error::NotFound => (StatusCode::NOT_FOUND, json!("404 Not Found")),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                serde_json::json!({
                    "error": {
                        "details": self.to_string()
                    }
                }),
            ),
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (code, json) = self.response_tuple();
        (code, Json(json)).into_response()
    }
}

impl From<biscuit_auth::error::Token> for Error {
    fn from(err: biscuit_auth::error::Token) -> Self {
        Error::AuthError(pic_store_auth::Error::TokenError(err))
    }
}
