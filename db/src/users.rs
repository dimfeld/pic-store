use diesel::prelude::*;
use serde::Deserialize;

use crate::{
    object_id::{TeamId, UserId},
    schema::*,
};

pub use crate::schema::users::*;

#[derive(Clone, Debug, Queryable, Identifiable)]
#[diesel(primary_key(user_id))]
pub struct User {
    pub user_id: UserId,
    pub team_id: TeamId,
    pub email: String,
    pub password_hash: Option<Vec<u8>>,
    pub name: String,
    pub updated: chrono::DateTime<chrono::Utc>,
    pub deleted: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Deserialize, Insertable)]
#[diesel(table_name = users)]
pub struct NewUser {
    pub user_id: UserId,
    pub team_id: TeamId,
    pub email: String,
    pub name: String,
}
