use diesel::prelude::*;
use serde::Deserialize;

use crate::{
    object_id::{TeamId, UploadProfileId, UserId},
    schema::*,
};

pub use crate::schema::users::*;

#[derive(Clone, Debug, Queryable, Identifiable)]
pub struct User {
    pub id: UserId,
    pub team_id: TeamId,
    pub email: String,
    pub password_hash: Option<String>,
    pub name: String,
    pub default_upload_profile_id: Option<UploadProfileId>,
    pub updated: chrono::DateTime<chrono::Utc>,
    pub deleted: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Deserialize, Insertable)]
#[diesel(table_name = users)]
pub struct NewUser {
    pub id: UserId,
    pub team_id: TeamId,
    pub email: String,
    pub name: String,
    pub password_hash: Option<String>,
    pub default_upload_profile_id: Option<UploadProfileId>,
}
