use http::uri::InvalidUri;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Unknown storage type {0}")]
    UnknownStorageType(String),

    #[error("Unsupported storage type {0}")]
    UnsupportedStorageType(String),

    #[error(transparent)]
    InvalidUri(#[from] InvalidUri),

    #[error("This storage provider does not support presigned URIs")]
    PresignedUriNotSupported,

    #[error("URI must be absolute")]
    RelativeUri,

    #[error("URI must have a path")]
    UriMissingPath,

    #[error(transparent)]
    PresignedUriCreation(#[from] anyhow::Error),
}
