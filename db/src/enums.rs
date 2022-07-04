use diesel::prelude::*;
use diesel_derive_enum::DbEnum;

#[derive(Debug, Clone, Copy, DbEnum)]
pub enum ImageFormat {
    Png,
    Jpg,
    Avif,
    Webp,
}

#[derive(Debug, Clone, Copy, DbEnum)]
pub enum BaseImageStatus {
    AwaitingUpload,
    Converting,
    Ready,
    QueuedForDelete,
    Deleting,
    Deleted,
}

impl Default for BaseImageStatus {
    fn default() -> Self {
        Self::AwaitingUpload
    }
}

#[derive(PartialEq, Eq, Copy, Clone, Debug, DbEnum)]
pub enum OutputImageStatus {
    Queued,
    Converting,
    Ready,
    QueuedForDelete,
    Deleted,
}

impl Default for OutputImageStatus {
    fn default() -> Self {
        Self::Queued
    }
}
