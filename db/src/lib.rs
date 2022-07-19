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

use async_trait::async_trait;
use diesel::{Connection, PgConnection};

pub type Pool = deadpool_diesel::postgres::Pool;

pub fn connect(conn_str: &str, max_connections: usize) -> Result<Pool, impl std::error::Error> {
    let manager =
        deadpool_diesel::postgres::Manager::new(conn_str, deadpool_diesel::Runtime::Tokio1);
    deadpool_diesel::Pool::builder(manager)
        .max_size(max_connections)
        .build()
}

pub fn new_uuid() -> uuid::Uuid {
    ulid::Ulid::new().into()
}

#[async_trait]
pub trait PoolExt<F, RETVAL, ERR>
where
    F: (FnOnce(&mut PgConnection) -> Result<RETVAL, ERR>) + Send + 'static,
    RETVAL: Send + 'static,
    ERR: Send + 'static,
{
    async fn interact(&self, f: F) -> Result<RETVAL, ERR>;
    async fn transaction(&self, f: F) -> Result<RETVAL, ERR>;
}

#[async_trait]
impl<F, RETVAL, ERR> PoolExt<F, RETVAL, ERR> for Pool
where
    F: (FnOnce(&mut PgConnection) -> Result<RETVAL, ERR>) + Send + 'static,
    RETVAL: Send + 'static,
    ERR: From<diesel::result::Error> + From<deadpool_diesel::PoolError> + Send + 'static,
{
    async fn interact(&self, f: F) -> Result<RETVAL, ERR> {
        let conn = self.get().await?;
        let result = conn.interact(move |conn| f(conn)).await.unwrap()?;
        Ok(result)
    }

    async fn transaction(&self, f: F) -> Result<RETVAL, ERR> {
        let conn = self.get().await?;
        let result = conn
            .interact(move |conn| conn.transaction(move |conn| f(conn)))
            .await
            .unwrap()?;
        Ok(result)
    }
}

sql_function! {
    #[aggregate]
    fn array_agg<X: diesel::sql_types::SingleValue>(x: X) -> diesel::sql_types::Array<X>
}
