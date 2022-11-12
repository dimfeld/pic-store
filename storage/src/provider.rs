use aws_sdk_s3::client::Client as S3Client;
use backon::ExponentialBackoff;
use opendal::{
    layers::{RetryLayer, TracingLayer},
    Operator,
};

use pic_store_db as db;

use crate::{error::Error, s3::S3ProviderConfig};

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
        let operator = match self {
            Self::S3 { config, .. } => {
                crate::s3::create_opendal_operator(config, base_location).await?
            }
            Self::Local => {
                let mut builder = opendal::services::fs::Builder::default();

                if !base_location.is_empty() {
                    builder.root(base_location);
                }

                let acc = builder.build()?;
                Operator::new(acc)
            }
        };

        let operator = operator
            .layer(RetryLayer::new(ExponentialBackoff::default()))
            .layer(TracingLayer);
        Ok(operator)
    }
}
