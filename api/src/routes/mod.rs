use axum::Router;

mod conversion_profile;
mod health;
mod image;
mod profile;
pub mod storage_location;

pub fn configure_routes(router: Router) -> Router {
    router
        .merge(health::configure())
        .merge(profile::configure())
        .merge(image::configure())
        .merge(conversion_profile::configure())
        .merge(storage_location::configure())
}
