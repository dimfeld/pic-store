/// For objects that are always bound to a project, list all the objects for this project.
#[macro_export]
macro_rules! list_project_objects {
    ($schema: ident, $state: expr, $user: expr, $output: ty, $project_id: ident, $permission: expr) => {{
        use $schema::dsl;
        $state.db.interact(move |conn| {
            $schema::table
                .select(<$output>::as_select())
                .filter(dsl::deleted.is_null())
                .filter(dsl::project_id.eq($project_id))
                .filter(db::obj_allowed!(
                    $user.team_id,
                    &$user.roles,
                    dsl::project_id,
                    Permission::ProjectRead
                ))
                .load::<$output>(conn)
                .map_err($crate::Error::from)
        })
    }};
}

/// For objects that can be team-wide, list all the objects for this project and all the team-wide objects too.
#[macro_export]
macro_rules! list_project_and_global_objects {
    ($schema: ident, $state: expr, $user: expr, $output: ty, $project_id: ident, $permission: expr) => {{
        use $schema::dsl;
        $state.db.interact(move |conn| {
            let q = $schema::table
                .select(<$output>::as_select())
                .filter(dsl::deleted.is_null())
                .into_boxed()
                .filter(db::obj_allowed_or_projectless!(
                    $user.team_id,
                    &$user.roles,
                    dsl::project_id,
                    Permission::ProjectRead
                ));

            let q = db::with_project_or_global!(q, $project_id);

            q.load::<$output>(conn).map_err($crate::Error::from)
        })
    }};
}

/// Get an object for the given project.
#[macro_export]
macro_rules! get_object {
    ($schema: ident, $state: expr, $user: expr, $output: ty, $id: ident, $project_id: ident, $permission: expr) => {{
        use $schema::dsl;
        $state.db.interact(move |conn| {
            $schema::table
                .select((
                    <$output>::as_select(),
                    db::obj_allowed!(
                        $user.team_id,
                        &$user.roles,
                        dsl::project_id.assume_not_null(),
                        $permission
                    ),
                ))
                .filter(dsl::id.eq($id))
                .filter(dsl::team_id.eq($user.team_id))
                .filter(dsl::project_id.eq($project_id))
                .first::<($output, bool)>(conn)
                .map_err(Error::from)
        })
    }};
}

/// Get an object that can be team-wide or bound to a project.
#[macro_export]
macro_rules! get_maybe_global_object {
    ($schema: ident, $state: expr, $user: expr, $output: ty, $id: ident, $project_id: ident, $permission: expr) => {{
        use $schema::dsl;
        $state.db.interact(move |conn| {
            $schema::table
                .select((
                    <$output>::as_select(),
                    db::obj_allowed_or_projectless!(
                        $user.team_id,
                        &$user.roles,
                        dsl::project_id.assume_not_null(),
                        $permission
                    ),
                ))
                .filter(dsl::id.eq($id))
                .filter(dsl::project_id.is_not_distinct_from($project_id))
                .filter(dsl::team_id.eq($user.team_id))
                .first::<($output, bool)>(conn)
                .map_err(Error::from)
        })
    }};
}

/// Write an object
#[macro_export]
macro_rules! write_object {
    ($schema: ident, $state: expr, $user: expr, $id: expr, $project_id: expr, $output: ty, $permission: expr, $sets: expr) => {{
        use $schema::dsl;
        $state.db.interact(move |conn| {
            $crate::auth::must_have_permission_on_project(conn, &$user, $project_id, $permission)?;
            diesel::update($schema::table)
                .filter(dsl::id.eq($id))
                .filter(dsl::project_id.is_not_distinct_from($project_id))
                .filter(dsl::team_id.eq($user.team_id))
                .set($sets)
                .returning(<$output>::as_select())
                .get_result::<$output>(conn)
                .map_err($crate::Error::from)
        })
    }};
}

/// Write an object that might be team-wide or bound to a project.
#[macro_export]
macro_rules! write_maybe_global_object {
    ($schema: ident, $state: expr, $user: expr, $id: expr, $project_id: expr, $output: ty, $permission: expr, $sets: expr) => {
        $crate::write_object!(
            $schema,
            $state,
            $user,
            $id,
            $project_id.unwrap_or_else(ProjectId::nil),
            $output,
            $permission,
            $sets
        )
    };
}

/// Delete an object
#[macro_export]
macro_rules! disable_object {
    ($schema: ident, $state: expr, $user: expr, $id: expr, $project_id: expr, $permission: expr) => {{
        use $schema::dsl;
        $state.db.interact(move |conn| {
            $crate::auth::must_have_permission_on_project(conn, &$user, $project_id, $permission)?;

            diesel::update($schema::table)
                .filter(dsl::id.eq($id))
                .filter(dsl::project_id.is_not_distinct_from($project_id))
                .filter(dsl::team_id.eq($user.team_id))
                .set(dsl::deleted.eq(Some(Utc::now())))
                .execute(conn)
                .map_err($crate::Error::from)
        })
    }};
}

/// Delete an object that might be team-wide or bound to a project.
#[macro_export]
macro_rules! disable_maybe_global_object {
    ($schema: ident, $state: expr, $user: expr, $id: expr, $project_id: ident, $permission: expr) => {{
        use $schema::dsl;
        $state.db.interact(move |conn| {
            $crate::auth::must_have_permission_on_project(
                conn,
                &$user,
                $project_id.unwrap_or_else(ProjectId::nil),
                $permission,
            )?;

            diesel::update($schema::table)
                .filter(dsl::id.eq($id))
                .filter(dsl::project_id.is_not_distinct_from($project_id))
                .filter(dsl::team_id.eq($user.team_id))
                .set(dsl::deleted.eq(Some(Utc::now())))
                .execute(conn)
                .map_err($crate::Error::from)
        })
    }};
}
