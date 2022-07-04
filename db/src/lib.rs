#[macro_use]
extern crate diesel;

mod enums;
mod json;
mod models;
mod schema;

pub use enums::*;
pub use json::*;
pub use models::*;
pub use schema::*;

pub type Pool = deadpool_diesel::postgres::Pool;

pub fn connect(conn_str: &str) -> Result<Pool, impl std::error::Error> {
    let manager =
        deadpool_diesel::postgres::Manager::new(conn_str, deadpool_diesel::Runtime::Tokio1);
    deadpool_diesel::Pool::builder(manager).build()
}
