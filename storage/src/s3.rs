use std::time::Duration;

use aws_sdk_s3::presigning::config::PresigningConfig;
use aws_sdk_s3::{client::Client as S3Client, Credentials};
use http::Uri;

use crate::error::Error;
use crate::presigned_url::PresignedUrl;
use crate::provider::Provider;

#[derive(Debug, Clone)]
pub struct S3ProviderConfig {
    pub endpoint: Option<Uri>,
    pub access_key_id: String,
    pub secret_key: String,
}

pub fn create_client(config: &S3ProviderConfig) -> S3Client {
    let mut builder = aws_sdk_s3::Config::builder().credentials_provider(Credentials::new(
        config.access_key_id.as_str(),
        config.secret_key.as_str(),
        None,
        None,
        "config",
    ));

    if let Some(endpoint) = config.endpoint.clone() {
        builder = builder.endpoint_resolver(aws_sdk_s3::Endpoint::mutable(endpoint));
    }

    let config = builder.build();

    S3Client::from_conf(config)
}

impl Provider {
    pub(crate) async fn create_s3_presigned_upload_url(
        &self,
        client: &S3Client,
        destination: &str,
    ) -> Result<crate::presigned_url::PresignedUrl, Error> {
        let uri = destination.parse::<Uri>()?;
        let host = uri.host().ok_or(Error::RelativeUri)?;
        let path = uri.path();

        if path.is_empty() {
            return Err(Error::UriMissingPath);
        }

        let presign_config = PresigningConfig::builder()
            .expires_in(Duration::from_secs(15 * 60))
            .build()
            .unwrap();

        let (req, _) = client
            .put_object()
            .bucket(host)
            .key(path)
            .presigned(presign_config)
            .await
            .map_err(|e| Error::PresignedUriCreation(anyhow::Error::from(e)))?
            .to_http_request(())
            .map_err(|e| Error::PresignedUriCreation(anyhow::Error::from(e)))?
            .into_parts();

        Ok(PresignedUrl {
            method: req.method,
            uri: req.uri,
            headers: req.headers,
        })
    }
}
