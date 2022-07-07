use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

use crate::{schema::*, Permission};

#[derive(Clone, Debug, Queryable, Identifiable, Insertable)]
#[diesel(primary_key(session_id))]
pub struct ApiKey {
    pub api_key_id: Uuid,
    pub name: String,
    pub prefix: String,
    pub hash: Vec<u8>,
    pub team_id: Uuid,
    pub user_id: Uuid,
    pub inherits_user_permissions: bool,
    pub created: DateTime<Utc>,
    pub expires: DateTime<Utc>,
}

#[derive(Clone, Debug, Queryable, Insertable)]
#[diesel(primary_key(team_id, api_key_id, project_id, permission))]
pub struct ApiKeyPermission {
    pub team_id: Uuid,
    pub api_key_id: Uuid,
    pub project_id: Option<Uuid>,
    pub permission: Permission,
}