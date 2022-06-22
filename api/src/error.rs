use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use sea_orm::DbErr;
use serde::Serialize;
use thiserror::Error;

use pic_store_db as db;

#[derive(Debug, Serialize)]
pub struct ErrorResponseData {
    error: ErrorDetails,
}

#[derive(Debug, Serialize)]
struct ErrorDetails {
    details: String,
}

impl ErrorResponseData {
    pub fn new(message: String) -> ErrorResponseData {
        ErrorResponseData {
            error: ErrorDetails { details: message },
        }
    }
}

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

    #[error("Storage provider {0} does not support pre-signed URLs")]
    NoUploadUrlError(db::storage_location::Provider),
}

impl Error {
    pub fn response_tuple(&self) -> (StatusCode, ErrorResponseData) {
        let status = match self {
            Error::NoUploadUrlError(_) => StatusCode::BAD_REQUEST,
            Error::Unauthorized => StatusCode::UNAUTHORIZED,
            Error::NotFound => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        (status, ErrorResponseData::new(self.to_string()))
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
