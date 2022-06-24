use aws_sdk_s3::client::Client as S3Client;
use backon::ExponentialBackoff;
use opendal::Operator;

use crate::{error::Error, s3::S3ProviderConfig, PresignedUrl};

#[derive(Debug, Clone)]
pub enum ProviderConfig {
    S3(S3ProviderConfig),
    Local,
}

impl ProviderConfig {
    pub fn from_db(
        provider_type: pic_store_db::storage_location::Provider,
        credentials: serde_json::Value,
    ) -> Result<ProviderConfig, Error> {
        match provider_type {
            pic_store_db::storage_location::Provider::S3 => {
                S3ProviderConfig::try_from(credentials).map(ProviderConfig::S3)
            }
            pic_store_db::storage_location::Provider::Local => Ok(Self::Local),
        }
    }
}

#[derive(Debug)]
pub enum Provider {
    S3 {
        config: S3ProviderConfig,
        client: S3Client,
    },
    Local,
}

impl Provider {
    pub fn new(config: ProviderConfig) -> Self {
        match config {
            ProviderConfig::S3(config) => {
                let client = crate::s3::create_client(&config);
                Provider::S3 { config, client }
            }
            ProviderConfig::Local => Provider::Local,
        }
    }

    pub async fn create_operator(&self, base_location: &str) -> Result<Operator, anyhow::Error> {
        let accessor = match self {
            Self::S3 { config, .. } => {
                crate::s3::create_opendal_accessor(config, base_location).await?
            }
            Self::Local => {
                let mut builder = opendal::services::fs::Backend::build();

                if !base_location.is_empty() {
                    builder.root(base_location);
                }

                builder.finish().await?
            }
        };

        let operator = Operator::new(accessor).with_backoff(ExponentialBackoff::default());
        Ok(operator)
    }

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
