use axum::Router;

mod health;
mod profile;

pub fn configure_routes(router: Router) -> Router {
    router
        .merge(health::configure())
        .merge(profile::configure())
}
