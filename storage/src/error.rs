use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Unknown storage type {0}")]
    UnknownStorageType(String),

    #[error("Unsupported storage type {0}")]
    UnsupportedStorageType(String),
}
