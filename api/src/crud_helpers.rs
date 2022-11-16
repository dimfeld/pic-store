#[macro_export]
macro_rules! disable_object {
    ($state: expr, $user: expr, $dsl: ident, $table: expr, $id: expr, $project_id: expr, $permission: expr) => {
        $state.db.interact(move |conn| {
            $crate::auth::must_have_permission_on_project(conn, &$user, $project_id, $permission)?;

            diesel::update($table)
                .filter(dsl::id.eq($id))
                .filter(dsl::project_id.is_not_distinct_from($project_id))
                .filter(dsl::team_id.eq($user.team_id))
                .set(dsl::deleted.eq(Some(Utc::now())))
                .execute(conn)
                .map_err($crate::Error::from)
        })
    };
}

#[macro_export]
macro_rules! disable_maybe_global_object {
    ($state: expr, $user: expr, $dsl: ident, $table: expr, $id: expr, $project_id: ident, $permission: expr) => {
        $state.db.interact(move |conn| {
            $crate::auth::must_have_permission_on_project(
                conn,
                &$user,
                $project_id.unwrap_or_else(ProjectId::nil),
                $permission,
            )?;

            diesel::update($table)
                .filter(dsl::id.eq($id))
                .filter(dsl::project_id.is_not_distinct_from($project_id))
                .filter(dsl::team_id.eq($user.team_id))
                .set(dsl::deleted.eq(Some(Utc::now())))
                .execute(conn)
                .map_err($crate::Error::from)
        })
    };
}
