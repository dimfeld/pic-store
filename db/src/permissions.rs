use diesel::{dsl::exists, prelude::*, PgConnection};

use crate::{
    object_id::{ProjectId, RoleId, TeamId},
    Permission,
};

#[derive(Debug, Copy, Clone)]
pub enum GlobalPermission {
    TeamWrite,
    TeamAdmin,
    ProjectCreate,
}

impl From<GlobalPermission> for crate::Permission {
    fn from(p: GlobalPermission) -> Self {
        match p {
            GlobalPermission::TeamWrite => Permission::TeamWrite,
            GlobalPermission::TeamAdmin => Permission::TeamAdmin,
            GlobalPermission::ProjectCreate => Permission::ProjectCreate,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum ProjectPermission {
    ProjectRead,
    ProjectWrite,
    ImageEdit,
    ImageCreate,
    ConversionProfileWrite,
    StorageLocationWrite,
}

impl From<ProjectPermission> for crate::Permission {
    fn from(p: ProjectPermission) -> Self {
        match p {
            ProjectPermission::ProjectRead => Permission::ProjectRead,
            ProjectPermission::ProjectWrite => Permission::ProjectWrite,
            ProjectPermission::ImageEdit => Permission::ImageEdit,
            ProjectPermission::ImageCreate => Permission::ImageCreate,
            ProjectPermission::ConversionProfileWrite => Permission::ConversionProfileWrite,
            ProjectPermission::StorageLocationWrite => Permission::StorageLocationWrite,
        }
    }
}

#[macro_export]
macro_rules! obj_allowed {
    ($team_id: expr, $roles: expr, $obj_project_field: expr, $permission: expr) => {
        diesel::dsl::exists(
            $crate::role_permissions::table.filter(
                $crate::role_permissions::team_id
                    .eq($team_id)
                    .and($crate::role_permissions::role_id.eq_any($roles))
                    .and($crate::role_permissions::permission.eq($permission))
                    .and($crate::role_permissions::project_id.eq($obj_project_field)),
            ),
        )
    };
}

#[macro_export]
macro_rules! obj_allowed_or_projectless {
    ($team_id: expr, $roles: expr, $obj_project_field: expr, $permission: expr) => {
        $obj_project_field.is_null().or($crate::obj_allowed!(
            $team_id,
            $roles,
            $obj_project_field,
            $permission
        ))
    };
}

#[macro_export]
macro_rules! has_projectless_permission {
    ($team_id: expr, $roles: expr, $permission: expr) => {
        $crate::role_permissions::table.filter(
            $crate::role_permissions::team_id
                .eq($team_id)
                .and($crate::role_permissions::role_id.eq_any($roles))
                .and(
                    $crate::role_permissions::permission
                        .eq(($permission as $crate::permissions::GlobalPermission).into()),
                ),
        )
    };
}

#[macro_export]
macro_rules! has_permission_on_project {
    ($team_id: expr, $roles: expr, $project_id: expr, $permission: expr) => {
        $crate::role_permissions::table.filter(
            $crate::role_permissions::team_id
                .eq($team_id)
                .and($crate::role_permissions::role_id.eq_any($roles))
                .and($crate::role_permissions::project_id.eq($project_id))
                .and(
                    $crate::role_permissions::permission
                        .eq(($permission as $crate::permissions::ProjectPermission).into()),
                ),
        )
    };
}

pub fn has_global_permission(
    conn: &mut PgConnection,
    team_id: TeamId,
    roles: &[RoleId],
    permission: GlobalPermission,
) -> Result<bool, diesel::result::Error> {
    let permission: Permission = permission.into();

    let allowed = crate::role_permissions::table
        .filter(
            crate::role_permissions::team_id
                .eq(team_id)
                .and(crate::role_permissions::role_id.eq_any(roles))
                .and(crate::role_permissions::permission.eq(permission)),
        )
        .select((1i32.into_sql::<diesel::sql_types::Integer>(),))
        .first::<(i32,)>(conn)
        .optional()?;

    Ok(allowed.is_some())
}

pub fn has_permission_on_project(
    conn: &mut PgConnection,
    team_id: TeamId,
    roles: &[RoleId],
    project_id: Option<ProjectId>,
    permission: ProjectPermission,
) -> Result<bool, diesel::result::Error> {
    let project_id = project_id.as_deref().copied().unwrap_or(uuid::Uuid::nil());
    let permission: Permission = permission.into();

    let allowed = crate::role_permissions::table
        .filter(
            crate::role_permissions::team_id
                .eq(team_id)
                .and(crate::role_permissions::role_id.eq_any(roles))
                .and(crate::role_permissions::project_id.eq(project_id))
                .and(crate::role_permissions::permission.eq(permission)),
        )
        .select((1i32.into_sql::<diesel::sql_types::Integer>(),))
        .first::<(i32,)>(conn)
        .optional()?;

    Ok(allowed.is_some())
}
