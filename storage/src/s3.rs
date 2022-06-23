use aws_sdk_s3::{client::Client as S3Client, Credentials};
use http::Uri;

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
