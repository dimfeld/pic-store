use http::{
    uri::{Authority, Scheme},
    Uri,
};
use object_store::aws::AmazonS3;
use tracing::{event, Level};

#[derive(Debug, Clone)]
pub struct S3ProviderConfig {
    pub endpoint: Option<Uri>,
    pub region: Option<String>,
    pub access_key_id: Option<String>,
    pub secret_key: Option<String>,
    pub virtual_host_style: Option<bool>,
}

pub(crate) fn create_store(
    config: &S3ProviderConfig,
    base_location: &str,
) -> Result<AmazonS3, eyre::Report> {
    let virtual_host_style = config.virtual_host_style.unwrap_or(false);

    if base_location.is_empty() {
        return Err(eyre::eyre!("base_location is required"));
    }

    // Get just the bucket name. The base path is handled in [[Operator]].
    let (bucket, _) = match base_location.find('/') {
        Some(slash_pos) => base_location.split_at(slash_pos),
        None => (base_location, ""),
    };

    let mut builder = object_store::aws::AmazonS3Builder::new()
        .with_virtual_hosted_style_request(virtual_host_style)
        .with_bucket_name(bucket);

    match (config.access_key_id.as_ref(), config.secret_key.as_ref()) {
        (Some(access_key_id), Some(secret_key)) => {
            builder = builder
                .with_access_key_id(access_key_id.as_str())
                .with_secret_access_key(secret_key.as_str());
        }
        (Some(_), None) | (None, Some(_)) => {
            return Err(eyre::eyre!(
                "access_key_id and secret_key must be both set or both unset"
            ))
        }
        (None, None) => {}
    };

    if let Some(endpoint) = config.endpoint.as_ref() {
        event!(Level::DEBUG, ?endpoint);
        let needs_scheme = endpoint.scheme().is_none();

        let e = if virtual_host_style {
            // When using virtual host style, object_store requires us to prepend the bucket name
            // to the endpoint.
            let parts = endpoint.to_owned().into_parts();
            let authority = parts
                .authority
                .unwrap_or_else(|| Authority::from_static("missing-host"));
            let new_domain = format!("{}.{}", bucket, authority.as_str());
            let scheme = parts.scheme.unwrap_or(Scheme::HTTPS);

            format!("{}://{}", scheme.as_str(), new_domain)
        } else if needs_scheme {
            // We tolerate a missing https:// in the endpoint, but object_store will panic without it.
            let parts = endpoint.to_owned().into_parts();
            format!("https://{}", parts.authority.unwrap().as_str())
        } else {
            endpoint.to_string()
        };
        event!(Level::DEBUG, endpoint=%e, "Creating S3 provider with custom endpoint");
        builder = builder.with_endpoint(e);
    }

    if let Some(region) = config.region.as_ref() {
        builder = builder.with_region(region.as_str());
    }

    let acc = builder.build()?;

    Ok(acc)
}
