use axum::Router;

use crate::shared_state::AppState;

mod conversion_profile;
mod health;
mod image;
pub mod storage_location;
mod upload_profile;

pub fn configure_routes(router: Router<AppState>) -> Router<AppState> {
    let api_routes = router
        .merge(health::configure())
        .merge(image::configure())
        .merge(upload_profile::configure())
        .merge(conversion_profile::configure())
        .merge(storage_location::configure());

    Router::new().nest("/api", api_routes)
}
