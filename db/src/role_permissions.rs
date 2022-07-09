use diesel::prelude::*;

use crate::{
    object_id::{ProjectId, RoleId, TeamId},
    schema::*,
    Permission,
};

pub use crate::schema::role_permissions::*;

#[derive(Queryable, Insertable, Debug)]
#[diesel(primary_key(team_id, role_id, project_id, permission))]
pub struct RolePermission {
    pub team_id: TeamId,
    pub role_id: RoleId,
    pub project_id: Option<ProjectId>,
    pub permission: Permission,
}
