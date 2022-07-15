#[macro_use]
extern crate diesel;

mod enums;
mod json;
mod schema;

pub mod api_keys;
pub mod base_images;
pub mod conversion_profiles;
pub mod object_id;
pub mod output_images;
pub mod permissions;
pub mod projects;
pub mod role_permissions;
pub mod roles;
pub mod sessions;
pub mod storage_locations;
pub mod teams;
pub mod upload_profiles;
pub mod user_roles;
pub mod users;

pub use enums::*;
pub use json::*;

pub type Pool = deadpool_diesel::postgres::Pool;

pub fn connect(conn_str: &str) -> Result<Pool, impl std::error::Error> {
    let manager =
        deadpool_diesel::postgres::Manager::new(conn_str, deadpool_diesel::Runtime::Tokio1);
    deadpool_diesel::Pool::builder(manager).build()
}

pub fn new_uuid() -> uuid::Uuid {
    ulid::Ulid::new().into()
}

sql_function! {
    #[aggregate]
    fn array_agg<X: diesel::sql_types::SingleValue>(x: X) -> diesel::sql_types::Array<X>
}
