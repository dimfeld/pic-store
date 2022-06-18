use axum::Router;

mod conversion_profile;
mod health;
mod profile;

pub fn configure_routes(router: Router) -> Router {
    router
        .merge(health::configure())
        .nest("/profiles", profile::configure())
        .nest("/conversion_profiles", conversion_profile::configure())
}
