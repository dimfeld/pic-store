use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use sea_orm::DbErr;
use thiserror::Error;

use pic_store_db as db;
use pic_store_http_errors::ErrorResponseData;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    DbErr(#[from] DbErr),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Auth error: {0}")]
    AuthError(#[from] pic_store_auth::Error),

    #[error("Storage error: {0}")]
    StorageError(#[from] pic_store_storage::Error),

    #[error("Not found")]
    NotFound,

    #[error("Storage provider {0} does not support pre-signed URLs")]
    NoUploadUrlError(db::storage_location::Provider),

    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),
}

impl Error {
    pub fn response_tuple(&self) -> (StatusCode, ErrorResponseData) {
        let status = match self {
            Error::NoUploadUrlError(_) => StatusCode::BAD_REQUEST,
            Error::Unauthorized => StatusCode::UNAUTHORIZED,
            Error::NotFound => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        (
            status,
            pic_store_http_errors::ErrorResponseData::new(self.to_string()),
        )
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
