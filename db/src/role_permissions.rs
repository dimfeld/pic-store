use diesel::prelude::*;
use serde::Deserialize;

use crate::{
    object_id::{ProjectId, RoleId, TeamId},
    schema::*,
};

pub use crate::schema::role_permissions::*;
pub use crate::Permission;

#[derive(Queryable, Insertable, Deserialize, Debug)]
#[diesel(primary_key(team_id, role_id, project_id, permission))]
pub struct RolePermission {
    pub team_id: TeamId,
    pub role_id: RoleId,
    pub project_id: Option<ProjectId>,
    pub permission: Permission,
}
