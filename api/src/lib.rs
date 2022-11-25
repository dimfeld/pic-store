pub mod api_key;
pub mod auth;
pub mod config;
mod crud_helpers;
pub mod error;
pub mod jobs;
pub mod obfuscate_errors;
pub mod panic_handler;
pub mod routes;
pub mod shared_state;
pub mod tracing_config;

use axum::{routing::IntoMakeService, Extension, Router};
use clap::Parser;
use futures::Future;
use hyper::server::conn::AddrIncoming;
use pic_store_db::object_id::{ProjectId, TeamId, UserId};
use std::{
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    sync::Arc,
    time::Duration,
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
    error::{Error, Result},
    obfuscate_errors::ObfuscateErrorLayer,
    shared_state::{AppState, InnerState},
    tracing_config::{HoneycombConfig, TracingExportConfig},
};

pub struct Server {
    pub host: String,
    pub port: u16,
    pub server: axum::Server<AddrIncoming, IntoMakeService<Router>>,
    pub state: Arc<InnerState>,
    pub worker: prefect::Worker,
}

impl Server {
    /// Run the server and wait for everything to close down once the server finishes.
    pub async fn run(self) -> Result<()> {
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

        tokio::task::spawn(async move {
            tokio::signal::ctrl_c()
                .await
                .expect("failed to listen for ctrl+c");
            shutdown_tx.send(()).ok();
        });

        self.run_with_shutdown_signal(shutdown_rx).await
    }

    pub async fn run_with_shutdown_signal<T>(
        self,
        shutdown_rx: impl Future<Output = T> + Send + 'static,
    ) -> Result<()> {
        let (internal_shutdown_tx, internal_shutdown_rx) = tokio::sync::oneshot::channel();

        tokio::task::spawn(async move {
            shutdown_rx.await;
            internal_shutdown_tx.send(()).ok();

            event!(Level::INFO, "Shutting down background jobs");
            if let Err(e) = self.worker.unregister(Some(Duration::from_secs(10))).await {
                event!(Level::ERROR, "Failed to shut down queue worker: {}", e);
            }
        });

        self.server
            .with_graceful_shutdown(async move {
                internal_shutdown_rx.await.ok();
                event!(Level::INFO, "Shutting down server");
            })
            .await
            .map_err(Error::ServerError)?;

        self.state.queue.close(Duration::from_secs(10)).await?;
        Ok(())
    }
}

pub async fn create_server(config: config::Config) -> Result<Server, eyre::Report> {
    let db = pic_store_db::connect(config.database_url.as_str(), 32)?;

    let production = config.env != "development" && !cfg!(debug_assertions);

    let (queue, worker) = jobs::create_job_queue(&PathBuf::from(config.queue_db_path), db.clone())
        .await
        .map_err(|e| eyre::eyre!("Failed to create job queue: {}", e))?;

    let state = Arc::new(InnerState {
        production,
        db: db.clone(),
        queue,
        // Temporary hardcoded values
        project_id: std::env::var("DEFAULT_PROJECT_ID")
            .expect("DEFAULT_PROJECT_ID")
            .parse::<ProjectId>()
            .unwrap(),
        team_id: std::env::var("DEFAULT_TEAM_ID")
            .expect("DEFAULT_TEAM_ID")
            .parse::<TeamId>()
            .unwrap(),
        user_id: std::env::var("DEFAULT_USER_ID")
            .expect("DEFAULT_USER_ID")
            .parse::<UserId>()
            .unwrap(),
    });

    let app: Router<AppState> = routes::configure_routes(Router::new()).layer(
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

    let app: Router<()> = app.with_state::<()>(state.clone());

    let bind_ip: IpAddr = config.host.parse()?;
    let bind_addr = SocketAddr::from((bind_ip, config.port));
    let builder = axum::Server::bind(&bind_addr);

    let server = builder.serve(app.into_make_service());
    let actual_addr = server.local_addr();
    let port = actual_addr.port();
    event!(Level::INFO, "Listening on {}:{port}", config.host);

    Ok(Server {
        host: config.host,
        port,
        server,
        state,
        worker,
    })
}
