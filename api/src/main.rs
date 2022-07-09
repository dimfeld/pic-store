mod auth;
mod config;
mod error;
mod obfuscate_errors;
mod panic_handler;
mod routes;
mod shared_state;
mod tracing_config;

use axum::{Extension, Router};
use clap::Parser;
use pic_store_db::object_id::{ProjectId, TeamId, UserId};
use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
};
use tower::ServiceBuilder;
use tower_cookies::CookieManagerLayer;
use tower_http::{
    catch_panic::CatchPanicLayer,
    request_id::MakeRequestUuid,
    trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer},
    ServiceBuilderExt,
};
use tracing::{event, Level};

use crate::{
    auth::auth_layer, error::Error, obfuscate_errors::ObfuscateErrorLayer,
    shared_state::InnerState, tracing_config::HoneycombConfig,
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

    let db = pic_store_db::connect(config.database_url.as_str())?;

    let production = config.env != "development" && !cfg!(debug_assertions);

    let state = Arc::new(InnerState {
        production,
        db: db.clone(),
        // Temporary hardcoded values
        project_id: std::env::var("PROJECT_ID")
            .expect("PROJECT_ID")
            .parse::<ProjectId>()
            .unwrap(),
        team_id: std::env::var("TEAM_ID")
            .expect("TEAM_ID")
            .parse::<TeamId>()
            .unwrap(),
        user_id: std::env::var("USER_ID")
            .expect("USER_ID")
            .parse::<UserId>()
            .unwrap(),
    });

    let app = routes::configure_routes(Router::new()).layer(
        // Global middlewares
        ServiceBuilder::new()
            .layer(CatchPanicLayer::custom(move |err| {
                panic_handler::handle_panic(production, err)
            }))
            .layer(ObfuscateErrorLayer::new(production))
            .compression()
            .decompression()
            .layer(CookieManagerLayer::new())
            .set_x_request_id(MakeRequestUuid)
            .propagate_x_request_id()
            .layer(Extension(state))
            .layer(auth_layer(
                db.clone(),
                config.session_cookie_name.clone(),
                &config.cookie_key,
            ))
            .layer(
                TraceLayer::new_for_http()
                    .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                    .on_response(DefaultOnResponse::new().level(Level::INFO))
                    .on_request(DefaultOnRequest::new().level(Level::INFO)),
            )
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
