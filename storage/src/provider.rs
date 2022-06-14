use std::sync::Arc;

use crate::Error;
use opendal::{Accessor, Operator};

#[derive(Debug, Default)]
pub struct S3Config {
    pub endpoint: Option<String>,
    pub access_key_id: Option<String>,
    pub secret_key: Option<String>,
}

pub struct StorageProviderConfig {
    pub s3: Option<S3Config>,
}

pub struct StorageProvider {
    config: StorageProviderConfig,
}

impl StorageProvider {
    pub fn operator_from_url(url: &str) -> Result<Operator, Error> {
        todo!();
    }
}
