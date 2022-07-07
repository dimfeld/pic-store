use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

use crate::{
    object_id::{ProjectId, ProjectId, RoleId, TeamId, UserId},
    schema::*,
    Permission,
};

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

#[derive(Debug, Insertable)]
#[diesel(table_name = users)]
pub struct NewUser {
    pub user_id: UserId,
    pub team_id: TeamId,
    pub email: String,
    pub name: String,
}

#[derive(Queryable, Identifiable, Debug)]
#[diesel(primary_key(role_id))]
pub struct Role {
    pub role_id: RoleId,
    pub team_id: TeamId,
    pub name: String,
    pub created: DateTime<Utc>,
}

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

#[derive(Queryable, Insertable, Debug)]
#[diesel(primary_key(team_id, role_id, project_id, permission))]
pub struct RolePermission {
    pub team_id: TeamId,
    pub role_id: RoleId,
    pub project_id: Option<ProjectId>,
    pub permission: Permission,
}
