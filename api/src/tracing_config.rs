use tracing::subscriber::set_global_default;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{fmt::MakeWriter, layer::SubscriberExt, EnvFilter, Registry};

pub fn configure<W>(name: impl Into<String>, sink: W) -> Result<(), anyhow::Error>
where
    W: for<'a> MakeWriter<'a> + 'static + Send + Sync,
{
    let oltp = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(opentelemetry_otlp::new_exporter().tonic())
        .install_batch(opentelemetry::runtime::TokioCurrentThread)?;
    let telemetry = tracing_opentelemetry::layer().with_tracer(oltp);

    LogTracer::builder()
        .ignore_crate("rustls")
        .with_max_level(log::LevelFilter::Debug)
        .init()
        .expect("Failed to create logger");

    let env_filter = EnvFilter::try_from_env("LOG").unwrap_or_else(|_| EnvFilter::new("info"));

    let formatting_layer = BunyanFormattingLayer::new(name.into(), sink);
    let subscriber = Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer)
        .with(telemetry);
    set_global_default(subscriber).expect("Setting subscriber");
    Ok(())
}
