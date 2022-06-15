mod config;
mod tracing_config;

use clap::Parser;
use tracing::{event, Level};

fn main() {
    let config = config::Config::parse();

    tracing_config::configure("pic-store-api", std::io::stdout).unwrap();

    event!(Level::INFO, "Listening on {}:{}", config.host, config.port);
}
