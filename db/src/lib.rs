#[macro_use]
extern crate diesel;

mod enums;
mod json;
mod schema;

pub mod api_key;
pub mod conversion_profiles;
pub mod images;
pub mod object_id;
pub mod projects;
pub mod session;
pub mod storage_locations;
pub mod teams;
pub mod upload_profiles;
pub mod users;

pub use enums::*;
pub use json::*;
pub use schema::*;

pub type Pool = deadpool_diesel::postgres::Pool;

pub fn connect(conn_str: &str) -> Result<Pool, impl std::error::Error> {
    let manager =
        deadpool_diesel::postgres::Manager::new(conn_str, deadpool_diesel::Runtime::Tokio1);
    deadpool_diesel::Pool::builder(manager).build()
}

pub fn new_uuid() -> uuid::Uuid {
    ulid::Ulid::new().into()
}
