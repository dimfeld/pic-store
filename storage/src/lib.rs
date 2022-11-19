mod error;
mod operator;
mod provider;
mod s3;

pub use error::*;
pub use operator::*;
pub use provider::*;

#[derive(Debug)]
pub struct PresignedUrl {
    pub method: http::Method,
    pub uri: http::Uri,
    pub headers: http::HeaderMap,
}
