mod config;
mod tracing_config;

use std::net::{IpAddr, SocketAddr};

use axum::Router;
use clap::Parser;
use tracing::{event, Level};

use crate::tracing_config::HoneycombConfig;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let mut config = config::Config::parse();

    let honeycomb_config = if let Some(team) = config.honeycomb_team.take() {
        Some(HoneycombConfig {
            team,
            dataset: std::mem::take(&mut config.honeycomb_dataset),
        })
    } else {
        None
    };

    tracing_config::configure("pic-store-api", std::io::stdout, honeycomb_config)?;

    let app = Router::new();

    let bind_ip: IpAddr = config.host.parse()?;
    let addr = SocketAddr::from((bind_ip, config.port));
    let builder = axum::Server::bind(&addr);
    event!(Level::INFO, "Listening on {}:{}", config.host, config.port);

    builder.serve(app.into_make_service()).await?;

    tracing_config::teardown();

    Ok(())
}
