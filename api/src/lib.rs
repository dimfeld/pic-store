pub mod auth;
pub mod config;
pub mod error;
pub mod jobs;
pub mod obfuscate_errors;
pub mod panic_handler;
pub mod routes;
pub mod shared_state;
pub mod tracing_config;

use axum::{routing::IntoMakeService, Extension, Router};
use clap::Parser;
use hyper::server::conn::AddrIncoming;
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
    auth::auth_layer,
    error::Error,
    obfuscate_errors::ObfuscateErrorLayer,
    shared_state::InnerState,
    tracing_config::{HoneycombConfig, TracingExportConfig},
};

pub struct Server {
    pub host: String,
    pub port: u16,
    pub server: axum::Server<AddrIncoming, IntoMakeService<Router>>,
}

pub async fn run_server(config: config::Config) -> Result<Server, anyhow::Error> {
    let db = pic_store_db::connect(config.database_url.as_str(), 32)?;

    let production = config.env != "development" && !cfg!(debug_assertions);

    let job_registry = jobs::create_registry(db.clone());
    let _job_worker =
        jobs::start_workers(config.database_url.clone(), db.clone(), job_registry, 5, 10).await?;

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
            .layer(ObfuscateErrorLayer::new(production, false))
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

    let server = builder.serve(app.into_make_service());

    Ok(Server {
        host: config.host,
        port: addr.port(),
        server,
    })
}
