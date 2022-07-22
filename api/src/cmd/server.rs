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

    pic_store_api::run_server(config).await?;

    tracing_config::teardown();

    Ok(())
}
