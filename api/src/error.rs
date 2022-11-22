use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use pic_store_db::Permission;
use thiserror::Error;

use pic_store_http_errors::ErrorResponseData;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Database Error: {0}")]
    DbErr(#[from] diesel::result::Error),

    #[error("Database Pool Error: {0}")]
    DbPool(#[from] deadpool_diesel::PoolError),

    #[error("Server error: {0}")]
    ServerError(hyper::Error),

    #[error("Database Error: {0}")]
    DeadpoolInteract(anyhow::Error),

    #[error("Missing Permission {0}")]
    MissingPermission(Permission),

    #[error("Unauthenticated")]
    Unauthenticated,

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

    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    AxumError(#[from] axum::Error),

    #[error("Failed to decode image information: {0}")]
    ImageHeaderDecode(#[from] imageinfo::ImageInfoError),

    #[error("Unsupported image type: {0:?}")]
    UnsupportedImageType(imageinfo::ImageFormat),

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

    #[error("Queue error: {0}")]
    Queue(#[from] prefect::Error),
}

impl Error {
    fn error_kind(&self) -> &'static str {
        match self {
            Error::DbErr(_) => "db",
            Error::DbPool(_) => "db_pool",
            Error::ServerError(_) => "internal_server_error",
            Error::DeadpoolInteract(_) => "db",
            Error::MissingPermission(_) => "missing_permission",
            Error::Unauthenticated => "authn",
            Error::AuthError(_) => "authz",
            Error::ApiKeyNotFound => "authn",
            Error::StorageError(_) => "storage",
            Error::NotFound => "not_found",
            Error::ObjectNotFound(_) => "not_found",
            Error::IoError(_) => "internal_server_error",
            Error::AxumError(_) => "bad_request",
            Error::ImageHeaderDecode(_) => "image_decode",
            Error::UnsupportedImageType(_) => "unsupported_image_type",
            Error::ContentLengthRequired => "bad_request",
            Error::RequestTooLarge => "bad_request",
            Error::Generic(_) => "internal_server_error",
            Error::InvalidSessionId => "authn",
            Error::NoUploadProfile => "no_upload_profile",
            Error::Queue(_) => "job_queue",
        }
    }

    pub fn response_tuple(&self) -> (StatusCode, ErrorResponseData) {
        let status = match self {
            Error::NoUploadProfile => StatusCode::BAD_REQUEST,
            Error::MissingPermission(_) => StatusCode::FORBIDDEN,
            Error::NotFound => StatusCode::NOT_FOUND,
            Error::Unauthenticated => StatusCode::FORBIDDEN,
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
            pic_store_http_errors::ErrorResponseData::new(self.error_kind(), self.to_string()),
        )
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
