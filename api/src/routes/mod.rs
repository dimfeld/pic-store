use axum::Router;

mod health;

pub fn configure_routes(router: Router) -> Router {
    router.merge(health::configure())
}
