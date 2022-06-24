pub mod error;
mod provider;
mod s3;

pub use error::*;
pub use provider::*;

#[derive(Debug)]
pub struct PresignedUrl {
    pub method: http::Method,
    pub uri: http::Uri,
    pub headers: http::HeaderMap,
}
