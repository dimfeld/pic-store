use pic_store_api::tracing_config::{self, HoneycombConfig, TracingExportConfig};

pub async fn run(
    mut config: pic_store_api::config::Config,
) -> Result<(), Box<dyn std::error::Error>> {
    let tracing_export_config = if let Some(team) = config.honeycomb_team.take() {
        TracingExportConfig::Honeycomb(HoneycombConfig {
            team,
            dataset: std::mem::take(&mut config.honeycomb_dataset),
        })
    } else if let Some(jaeger_endpoint) = config.jaeger_endpoint.take() {
        TracingExportConfig::Jaeger(jaeger_endpoint)
    } else {
        TracingExportConfig::None
    };

    tracing_config::configure(tracing_export_config)?;

    let server = pic_store_api::create_server(config).await?;
    let result = server.run().await;

    tracing_config::teardown();

    result?;
    Ok(())
}
