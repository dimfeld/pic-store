use aws_sdk_s3::client::Client as S3Client;
use backon::ExponentialBackoff;
use opendal::Operator;

use pic_store_db as db;

use crate::{error::Error, s3::S3ProviderConfig, PresignedUrl};

#[derive(Debug, Clone)]
pub enum ProviderConfig {
    S3(S3ProviderConfig),
    Local,
}

impl ProviderConfig {
    pub fn from_db(
        provider_type: db::storage_locations::Provider,
    ) -> Result<ProviderConfig, Error> {
        match provider_type {
            db::storage_locations::Provider::S3 {
                endpoint,
                access_key_id,
                secret_key,
            } => {
                let uri = endpoint.map(|ep| ep.parse::<http::Uri>()).transpose()?;

                Ok(ProviderConfig::S3(S3ProviderConfig {
                    endpoint: uri,
                    access_key_id,
                    secret_key,
                }))
            }
            db::storage_locations::Provider::Local => Ok(Self::Local),
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

    pub fn from_db(provider_type: db::storage_locations::Provider) -> Result<Self, Error> {
        let config = ProviderConfig::from_db(provider_type)?;
        Ok(Provider::new(config))
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
