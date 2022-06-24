pub mod error;
pub mod provider;
mod s3;

pub struct PresignedUrl {
    pub method: http::Method,
    pub uri: http::Uri,
    pub headers: http::HeaderMap,
}
