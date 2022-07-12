#[macro_export]
macro_rules! allowed {
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
macro_rules! allowed_or_projectless {
    ($team_id: expr, $roles: expr, $obj_project_field: expr, $permission: expr) => {
        $obj_project_field.is_null().or($crate::allowed!(
            $team_id,
            $roles,
            $obj_project_field,
            $permission
        ))
    };
}
