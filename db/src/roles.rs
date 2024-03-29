use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::Deserialize;

use crate::{
    object_id::{RoleId, TeamId},
    schema::*,
};

pub use crate::schema::roles::*;

#[derive(Queryable, Identifiable, Debug)]
pub struct Role {
    pub id: RoleId,
    pub team_id: TeamId,
    pub name: String,
    pub created: DateTime<Utc>,
}

#[derive(Insertable, Deserialize, Debug)]
#[diesel(table_name = roles)]
pub struct NewRole {
    pub id: RoleId,
    pub team_id: TeamId,
    pub name: String,
}
