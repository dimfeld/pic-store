use std::sync::Arc;

use object_store::{local::LocalFileSystem, ObjectStore};

use pic_store_db as db;

use crate::{error::Error, s3::S3ProviderConfig, Operator};

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
                region,
                access_key_id,
                secret_key,
                virtual_host_style,
            } => {
                let uri = endpoint.map(|ep| ep.parse::<http::Uri>()).transpose()?;

                Ok(ProviderConfig::S3(S3ProviderConfig {
                    endpoint: uri,
                    region,
                    access_key_id,
                    secret_key,
                    virtual_host_style,
                }))
            }
            db::storage_locations::Provider::Local => Ok(Self::Local),
        }
    }
}

#[derive(Debug)]
pub enum Provider {
    S3 { config: S3ProviderConfig },
    Local,
}

impl Provider {
    pub fn new(config: ProviderConfig) -> Self {
        match config {
            ProviderConfig::S3(config) => Provider::S3 { config },
            ProviderConfig::Local => Provider::Local,
        }
    }

    pub fn from_db(provider_type: db::storage_locations::Provider) -> Result<Self, Error> {
        let config = ProviderConfig::from_db(provider_type)?;
        Ok(Provider::new(config))
    }

    pub async fn create_operator(&self, base_location: &str) -> Result<Operator, eyre::Report> {
        let (operator, supports_multipart, manual_prefix): (Box<dyn ObjectStore>, bool, bool) =
            match self {
                Self::S3 { config, .. } => (
                    Box::new(crate::s3::create_store(config, base_location)?),
                    true,
                    true,
                ),
                Self::Local => {
                    let store = if !base_location.is_empty() {
                        let path = std::path::PathBuf::from(base_location);
                        if !path.is_absolute() {
                            let full_path = path.canonicalize()?;
                            LocalFileSystem::new_with_prefix(&full_path)?
                        } else {
                            LocalFileSystem::new_with_prefix(&path)?
                        }
                    } else {
                        LocalFileSystem::new()
                    };

                    (Box::new(store), false, false)
                }
            };

        let path_prefix = if manual_prefix {
            None
        } else {
            Some(object_store::path::Path::from(base_location))
        };

        Ok(Operator {
            operator,
            supports_multipart,
            path_prefix,
        })
    }
}
