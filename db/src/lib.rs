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
pub mod test;
pub mod upload_profiles;
pub mod user_roles;
pub mod users;

use std::borrow::Cow;

use async_trait::async_trait;
use diesel::{sql_types, Connection, PgConnection};
pub use enums::*;
pub use json::*;

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

#[macro_export]
macro_rules! with_project_or_global {
    ($query: expr,  $project_id: expr) => {
        if let Some(project_id) = $project_id {
            $query.filter(
                dsl::project_id
                    .is_null()
                    .or(dsl::project_id.is_not_distinct_from(project_id)),
            )
        } else {
            $query.filter(dsl::project_id.is_null())
        }
    };
}

sql_function! {
    #[aggregate]
    fn array_agg<X: sql_types::SingleValue>(x: X) -> sql_types::Array<X>
}

sql_function! {
    fn coalesce<X: sql_types::SingleValue>(x: sql_types::Nullable<X>, y: sql_types::Nullable<X>) -> sql_types::Nullable<X>
}

sql_function! {
    #[aggregate]
    fn bool_or(x: sql_types::Bool) -> sql_types::Bool
}

pub fn image_path(
    storage_location_path: &str,
    project_base_path: &str,
    profile_path: &Option<String>,
    image_location: &str,
) -> String {
    match (project_base_path, profile_path) {
        ("", Some(p2)) => format!("{}/{}/{}", storage_location_path, p2, image_location),
        ("", None) => format!("{}/{}", storage_location_path, image_location),
        (p1, Some(p2)) => {
            format!("{}/{}/{}/{}", storage_location_path, p1, p2, image_location)
        }
        (p1, None) => format!("{}/{}/{}", storage_location_path, p1, image_location),
    }
}

pub fn image_base_location<'a, 'b>(
    storage_location_path: &'a str,
    project_base_path: &'b str,
    profile_path: &'b Option<String>,
) -> Cow<'a, str> {
    match (project_base_path, profile_path) {
        ("", Some(p2)) => format!("{}/{}", storage_location_path, p2).into(),
        ("", None) => storage_location_path.into(),
        (p1, Some(p2)) => format!("{}/{}/{}", storage_location_path, p1, p2).into(),
        (p1, None) => format!("{}/{}", storage_location_path, p1).into(),
    }
}
