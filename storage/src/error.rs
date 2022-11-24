use http::uri::InvalidUri;
use thiserror::Error;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Unknown storage type {0}")]
    UnknownStorageType(String),

    #[error("Unsupported storage type {0}")]
    UnsupportedStorageType(String),

    #[error(transparent)]
    InvalidUri(#[from] InvalidUri),

    #[error("URI must be absolute")]
    UriMustBeAbsolute,

    #[error("URI must have a path")]
    UriMissingPath,

    #[error("Missing field {0}")]
    MissingField(&'static str),

    #[error("Operator error {0}")]
    OperatorError(#[from] object_store::Error),
}
