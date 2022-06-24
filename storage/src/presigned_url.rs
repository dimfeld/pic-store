use crate::{error::Error, provider::Provider};

pub struct PresignedUrl {
    pub method: http::Method,
    pub uri: http::Uri,
    pub headers: http::HeaderMap,
}

impl Provider {
    pub async fn create_presigned_upload_url(
        &self,
        destination: &str,
    ) -> Result<PresignedUrl, Error> {
        match self {
            Self::Local => Err(Error::PresignedUriNotSupported),
            Self::S3 { client, .. } => {
                self.create_s3_presigned_upload_url(client, destination)
                    .await
            }
        }
    }
}
