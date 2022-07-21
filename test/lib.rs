use std::{future::Future, time::Duration};

use once_cell::sync::Lazy;
use tracing::subscriber::set_global_default;
use tracing_error::ErrorLayer;
use tracing_log::LogTracer;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};
use tracing_tree::HierarchicalLayer;

fn configure_tracing() {
    LogTracer::builder()
        .ignore_crate("rustls")
        .with_max_level(log::LevelFilter::Debug)
        .init()
        .expect("Failed to create logger");

    let env_filter = EnvFilter::try_from_env("LOG").unwrap_or_else(|_| EnvFilter::new("info"));

    let tree = HierarchicalLayer::new(2)
        .with_targets(true)
        .with_bracketed_fields(true);

    let subscriber = Registry::default()
        .with(env_filter)
        .with(tree)
        .with(ErrorLayer::default());
    set_global_default(subscriber).expect("Setting subscriber");
}

pub static TRACING: Lazy<()> = Lazy::new(|| {
    if std::env::var("TEST_LOG").is_ok() {
        configure_tracing();
    }
});

/// Wait for a function to return a non-None value. If it tries more than 30 times
/// it will return an Err. The Err currently always indicates a time out.
pub async fn wait_for<Fut, DATA>(f: impl Fn() -> Fut) -> Result<DATA, ()>
where
    Fut: Future<Output = Option<DATA>>,
    DATA: Send + Sync,
{
    let mut tries = 0;
    while tries < 30 {
        match f().await {
            Some(d) => return Ok(d),
            None => {
                tokio::time::sleep(Duration::from_millis(250)).await;
                tries += 1;
            }
        }
    }

    Err(())
}
