use thiserror::Error;

use crate::EncodeError;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Reading {format:?}: {error}")]
    Read {
        format: Option<imageinfo::ImageFormat>,
        error: eyre::Report,
    },
    #[error(transparent)]
    Encode(#[from] EncodeError),
}

impl Error {
    pub fn read_error(format: Option<imageinfo::ImageFormat>, e: impl Into<eyre::Report>) -> Self {
        Self::Read {
            format,
            error: e.into(),
        }
    }
}
