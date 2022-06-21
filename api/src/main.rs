mod config;
mod error;
mod obfuscate_errors;
mod panic_handler;
mod routes;
mod shared_state;
mod tracing_config;

pub use error::Error;
use pic_store_auth::{keypair_from_priv_key_base64, RootAuthEvaulator};

use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
};

use axum::{Extension, Router};
use clap::Parser;
use tower::ServiceBuilder;
use tower_http::{
    catch_panic::CatchPanicLayer,
    request_id::MakeRequestUuid,
    trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer},
    ServiceBuilderExt,
};
use tracing::{event, Level};

use crate::{
    obfuscate_errors::ObfuscateErrorLayer, shared_state::InnerState,
    tracing_config::HoneycombConfig,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    color_eyre::install()?;
    dotenv::dotenv().ok();
    let mut config = config::Config::parse();

    let honeycomb_config = if let Some(team) = config.honeycomb_team.take() {
        Some(HoneycombConfig {
            team,
            dataset: std::mem::take(&mut config.honeycomb_dataset),
        })
    } else {
        None
    };

    tracing_config::configure(honeycomb_config)?;

    let db = pic_store_db::connect(config.database_url.as_str()).await?;

    let production = config.env != "development" && !cfg!(debug_assertions);

    let state = Arc::new(InnerState {
        production,
        db,
        auth: RootAuthEvaulator::new(),
    });

    let biscuit_keypair = keypair_from_priv_key_base64(config.biscuit_key.as_str())?;

    let app = routes::configure_routes(Router::new()).layer(
        // Global middlewares
        ServiceBuilder::new()
            .layer(CatchPanicLayer::custom(move |err| {
                panic_handler::handle_panic(production, err)
            }))
            .layer(ObfuscateErrorLayer::new(production))
            .compression()
            .decompression()
            .set_x_request_id(MakeRequestUuid)
            .propagate_x_request_id()
            .layer(Extension(state))
            .layer(
                TraceLayer::new_for_http()
                    .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                    .on_response(DefaultOnResponse::new().level(Level::INFO))
                    .on_request(DefaultOnRequest::new().level(Level::INFO)),
            )
            .layer(pic_store_auth::BiscuitExtractorLayer::with_pubkey(
                biscuit_keypair.public(),
            ))
            .into_inner(),
    );

    let bind_ip: IpAddr = config.host.parse()?;
    let addr = SocketAddr::from((bind_ip, config.port));
    let builder = axum::Server::bind(&addr);
    event!(Level::INFO, "Listening on {}:{}", config.host, config.port);

    builder.serve(app.into_make_service()).await?;

    tracing_config::teardown();

    Ok(())
}
