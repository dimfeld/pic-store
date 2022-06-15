use opentelemetry_otlp::WithExportConfig;
use tracing::subscriber::set_global_default;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{fmt::MakeWriter, layer::SubscriberExt, EnvFilter, Registry};

pub struct HoneycombConfig {
    pub team: String,
    pub dataset: String,
}

pub fn configure<W>(
    name: impl Into<String>,
    console_sink: W,
    honeycomb_config: Option<HoneycombConfig>,
) -> Result<(), anyhow::Error>
where
    W: for<'a> MakeWriter<'a> + 'static + Send + Sync,
{
    LogTracer::builder()
        .ignore_crate("rustls")
        .with_max_level(log::LevelFilter::Debug)
        .init()
        .expect("Failed to create logger");

    let env_filter = EnvFilter::try_from_env("LOG").unwrap_or_else(|_| EnvFilter::new("info"));

    let formatting_layer = BunyanFormattingLayer::new(name.into(), console_sink);
    let subscriber = Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer);

    if let Some(honeycomb_config) = honeycomb_config {
        let mut oltp_meta = tonic::metadata::MetadataMap::new();
        oltp_meta.insert("x-honeycomb-team", honeycomb_config.team.parse()?);

        let exporter = opentelemetry_otlp::new_exporter()
            .tonic()
            .with_endpoint("api.honeycomb.io:443")
            .with_metadata(oltp_meta);

        let oltp = opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_trace_config(opentelemetry::sdk::trace::config().with_resource(
                opentelemetry::sdk::Resource::new(vec![opentelemetry::KeyValue::new(
                    "service.name",
                    honeycomb_config.dataset,
                )]),
            ))
            .with_exporter(exporter)
            .install_batch(opentelemetry::runtime::TokioCurrentThread)?;
        let telemetry = tracing_opentelemetry::layer().with_tracer(oltp);

        let subscriber = subscriber.with(telemetry);
        set_global_default(subscriber).expect("Setting subscriber");
    } else {
        set_global_default(subscriber).expect("Setting subscriber");
    }

    Ok(())
}

pub fn teardown() {
    opentelemetry::global::shutdown_tracer_provider();
}
