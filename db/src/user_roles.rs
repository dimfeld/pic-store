use chrono::{DateTime, Utc};
use diesel::prelude::*;

use crate::{
    object_id::{RoleId, UserId},
    schema::*,
};

pub use crate::schema::user_roles::*;

#[derive(Queryable, Identifiable, Debug)]
#[diesel(primary_key(role_id, user_id))]
pub struct UserRole {
    pub role_id: RoleId,
    pub user_id: UserId,
    pub added: DateTime<Utc>,
}

#[derive(Queryable, Insertable, Debug)]
#[diesel(table_name = user_roles, primary_key(role_id, user_id))]
pub struct UserAndRole {
    pub role_id: RoleId,
    pub user_id: UserId,
}
