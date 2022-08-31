use anyhow::anyhow;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use pic_store_db::Permission;
use thiserror::Error;

use pic_store_http_errors::ErrorResponseData;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Database Error: {0}")]
    DbErr(#[from] diesel::result::Error),

    #[error("Database Pool Error: {0}")]
    DbPool(#[from] deadpool_diesel::PoolError),

    #[error("Database Error: {0}")]
    DeadpoolInteract(anyhow::Error),

    #[error("Queue Listener Error: {0}")]
    QueueListener(anyhow::Error),

    #[error("Missing Permission {0}")]
    MissingPermission(Permission),

    #[error("Auth error: {0}")]
    AuthError(#[from] pic_store_auth::Error),

    #[error("API Key Not Found")]
    ApiKeyNotFound,

    #[error("Storage error: {0}")]
    StorageError(#[from] pic_store_storage::Error),

    #[error("Not found")]
    NotFound,

    #[error("Unknown {0}")]
    ObjectNotFound(&'static str),

    #[error("Storage provider does not support pre-signed URLs")]
    NoUploadUrlError,

    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    AxumError(#[from] axum::Error),

    #[error("Failed to decode image information: {0}")]
    ImageHeaderDecode(#[from] imageinfo::ImageInfoError),

    #[error("content-length header is required")]
    ContentLengthRequired,

    #[error("request too large")]
    RequestTooLarge,

    #[error(transparent)]
    Generic(#[from] anyhow::Error),

    #[error("Invalid session id")]
    InvalidSessionId,

    #[error("Upload profile not specified and there is no default setting")]
    NoUploadProfile,
}

impl Error {
    pub fn from_presign_error(err: std::io::Error) -> Self {
        match err.kind() {
            std::io::ErrorKind::Unsupported => Self::NoUploadUrlError,
            _ => Self::from(err),
        }
    }

    pub fn response_tuple(&self) -> (StatusCode, ErrorResponseData) {
        let status = match self {
            Error::NoUploadUrlError => StatusCode::BAD_REQUEST,
            Error::NoUploadProfile => StatusCode::BAD_REQUEST,
            Error::MissingPermission(_) => StatusCode::FORBIDDEN,
            Error::NotFound => StatusCode::NOT_FOUND,
            Error::AuthError(_) => StatusCode::UNAUTHORIZED,
            Error::ApiKeyNotFound => StatusCode::UNAUTHORIZED,
            Error::InvalidSessionId => StatusCode::UNAUTHORIZED,
            Error::ObjectNotFound(_) => StatusCode::NOT_FOUND,
            Error::ContentLengthRequired => StatusCode::BAD_REQUEST,
            Error::RequestTooLarge => StatusCode::BAD_REQUEST,
            Error::ImageHeaderDecode(imageinfo::ImageInfoError::UnrecognizedFormat) => {
                StatusCode::BAD_REQUEST
            }
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        (
            status,
            pic_store_http_errors::ErrorResponseData::new(self.to_string()),
        )
    }
}

impl From<sqlxmq::Error> for Error {
    fn from(s: sqlxmq::Error) -> Self {
        match s {
            sqlxmq::Error::Pool(e) => Self::from(e),
            sqlxmq::Error::Diesel(e) => Self::from(e),
            sqlxmq::Error::Interact(e) => panic!("{}", e),
            sqlxmq::Error::ListenerError(e) => Self::QueueListener(anyhow::Error::new(e)),
        }
    }
}

impl From<deadpool_diesel::InteractError> for Error {
    fn from(e: deadpool_diesel::InteractError) -> Self {
        std::panic::panic_any(e)
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (code, json) = self.response_tuple();
        (code, Json(json)).into_response()
    }
}
