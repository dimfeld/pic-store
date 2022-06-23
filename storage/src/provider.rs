use aws_sdk_s3::client::Client as S3Client;
use backon::ExponentialBackoff;
use opendal::Operator;

use crate::s3::S3ProviderConfig;

#[derive(Debug, Clone)]
pub enum ProviderConfig {
    S3(S3ProviderConfig),
    Local,
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
            Self::S3 {
                config:
                    S3ProviderConfig {
                        endpoint,
                        secret_key,
                        access_key_id,
                    },
                ..
            } => {
                let mut builder = opendal::services::s3::Backend::build();
                builder
                    .access_key_id(access_key_id)
                    .secret_access_key(secret_key);
                if let Some(endpoint) = endpoint {
                    let e = endpoint.to_string();
                    builder.endpoint(e.as_str());
                }

                if !base_location.is_empty() {
                    let (bucket, root) = match base_location.find('/') {
                        Some(slash_pos) => base_location.split_at(slash_pos),
                        None => (base_location, ""),
                    };

                    builder.bucket(bucket);
                    if !root.is_empty() {
                        builder.root(root);
                    }
                }

                builder.finish().await?
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
}
