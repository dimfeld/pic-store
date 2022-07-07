use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

use crate::{
    object_id::{ProjectId, TeamId, UserId},
    schema::*,
    Permission,
};

pub use crate::schema::api_keys::*;

#[derive(Clone, Debug, Queryable, Identifiable, Insertable)]
#[diesel(primary_key(api_key_id))]
pub struct ApiKey {
    pub api_key_id: Uuid,
    pub name: String,
    pub prefix: String,
    pub hash: Vec<u8>,
    pub team_id: TeamId,
    pub user_id: UserId,
    pub inherits_user_permissions: bool,
    pub created: DateTime<Utc>,
    pub expires: DateTime<Utc>,
}

#[derive(Clone, Debug, Queryable, Insertable)]
#[diesel(primary_key(team_id, api_key_id, project_id, permission))]
pub struct ApiKeyPermission {
    pub team_id: TeamId,
    pub api_key_id: Uuid,
    pub project_id: Option<ProjectId>,
    pub permission: Permission,
}
