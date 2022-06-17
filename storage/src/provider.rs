use backon::ExponentialBackoff;
use opendal::Operator;

#[derive(Debug)]
pub enum StorageProviderConfig {
    S3 {
        endpoint: Option<String>,
        access_key_id: Option<String>,
        secret_key: Option<String>,
    },
    Local,
}

pub async fn create_operator(
    config: StorageProviderConfig,
    base_location: &str,
) -> Result<Operator, anyhow::Error> {
    let accessor = match config {
        StorageProviderConfig::S3 {
            endpoint,
            secret_key,
            access_key_id,
        } => {
            let mut builder = opendal::services::s3::Backend::build();
            if let Some(endpoint) = endpoint.as_deref() {
                builder.endpoint(endpoint);
            }

            if let Some(access_key_id) = access_key_id.as_deref() {
                builder.access_key_id(access_key_id);
            }

            if let Some(secret_key) = secret_key.as_deref() {
                builder.access_key_id(secret_key);
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
        StorageProviderConfig::Local => {
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
