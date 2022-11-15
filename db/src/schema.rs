// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "base_image_status"))]
    pub struct BaseImageStatus;

    #[derive(diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "image_format"))]
    pub struct ImageFormat;

    #[derive(diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "output_image_status"))]
    pub struct OutputImageStatus;

    #[derive(diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "permission"))]
    pub struct Permission;
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::*;
    use super::sql_types::Permission;

    api_key_permissions (team_id, api_key_id, project_id, permission) {
        team_id -> Uuid,
        api_key_id -> Uuid,
        project_id -> Uuid,
        permission -> Permission,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::*;

    api_keys (id) {
        id -> Uuid,
        name -> Text,
        prefix -> Text,
        hash -> Bytea,
        team_id -> Uuid,
        user_id -> Uuid,
        default_upload_profile_id -> Nullable<Uuid>,
        inherits_user_permissions -> Bool,
        created -> Timestamptz,
        expires -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::*;
    use super::sql_types::ImageFormat;
    use super::sql_types::BaseImageStatus;

    base_images (id) {
        id -> Uuid,
        team_id -> Uuid,
        project_id -> Uuid,
        user_id -> Uuid,
        hash -> Nullable<Text>,
        filename -> Text,
        location -> Text,
        width -> Int4,
        height -> Int4,
        format -> Nullable<ImageFormat>,
        upload_profile_id -> Uuid,
        status -> BaseImageStatus,
        alt_text -> Text,
        placeholder -> Nullable<Text>,
        updated -> Timestamptz,
        deleted -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::*;

    conversion_profiles (id) {
        id -> Uuid,
        team_id -> Uuid,
        project_id -> Nullable<Uuid>,
        name -> Text,
        output -> Jsonb,
        updated -> Timestamptz,
        deleted -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::*;
    use super::sql_types::OutputImageStatus;

    output_images (id) {
        id -> Uuid,
        team_id -> Uuid,
        base_image_id -> Uuid,
        location -> Text,
        width -> Int4,
        height -> Int4,
        size -> Jsonb,
        format -> Jsonb,
        status -> OutputImageStatus,
        updated -> Timestamptz,
        deleted -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::*;

    projects (id) {
        id -> Uuid,
        team_id -> Uuid,
        name -> Text,
        base_location -> Text,
        updated -> Timestamptz,
        deleted -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::*;
    use super::sql_types::Permission;

    role_permissions (team_id, role_id, project_id, permission) {
        team_id -> Uuid,
        role_id -> Uuid,
        project_id -> Uuid,
        permission -> Permission,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::*;

    roles (id) {
        id -> Uuid,
        team_id -> Uuid,
        name -> Text,
        created -> Timestamptz,
        deleted -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::*;

    sessions (id) {
        id -> Uuid,
        user_id -> Uuid,
        expires -> Timestamptz,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::*;

    storage_locations (id) {
        id -> Uuid,
        team_id -> Uuid,
        project_id -> Nullable<Uuid>,
        name -> Text,
        provider -> Jsonb,
        base_location -> Text,
        public_url_base -> Text,
        updated -> Timestamptz,
        deleted -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::*;

    teams (id) {
        id -> Uuid,
        name -> Text,
        deleted -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::*;

    upload_profiles (id) {
        id -> Uuid,
        team_id -> Uuid,
        project_id -> Uuid,
        name -> Text,
        short_id -> Nullable<Text>,
        base_storage_location_id -> Uuid,
        output_storage_location_id -> Uuid,
        conversion_profile_id -> Uuid,
        updated -> Timestamptz,
        deleted -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::*;

    user_roles (user_id, role_id) {
        user_id -> Uuid,
        role_id -> Uuid,
        added -> Timestamptz,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::enums::*;

    users (id) {
        id -> Uuid,
        team_id -> Uuid,
        email -> Text,
        password_hash -> Nullable<Text>,
        name -> Text,
        default_upload_profile_id -> Nullable<Uuid>,
        updated -> Timestamptz,
        deleted -> Nullable<Timestamptz>,
    }
}

diesel::joinable!(api_key_permissions -> api_keys (api_key_id));
diesel::joinable!(api_key_permissions -> teams (team_id));
diesel::joinable!(api_keys -> teams (team_id));
diesel::joinable!(api_keys -> upload_profiles (default_upload_profile_id));
diesel::joinable!(api_keys -> users (user_id));
diesel::joinable!(base_images -> projects (project_id));
diesel::joinable!(base_images -> teams (team_id));
diesel::joinable!(base_images -> upload_profiles (upload_profile_id));
diesel::joinable!(base_images -> users (user_id));
diesel::joinable!(conversion_profiles -> projects (project_id));
diesel::joinable!(conversion_profiles -> teams (team_id));
diesel::joinable!(output_images -> base_images (base_image_id));
diesel::joinable!(output_images -> teams (team_id));
diesel::joinable!(projects -> teams (team_id));
diesel::joinable!(role_permissions -> roles (role_id));
diesel::joinable!(role_permissions -> teams (team_id));
diesel::joinable!(roles -> teams (team_id));
diesel::joinable!(sessions -> users (user_id));
diesel::joinable!(storage_locations -> projects (project_id));
diesel::joinable!(storage_locations -> teams (team_id));
diesel::joinable!(upload_profiles -> conversion_profiles (conversion_profile_id));
diesel::joinable!(upload_profiles -> projects (project_id));
diesel::joinable!(upload_profiles -> teams (team_id));
diesel::joinable!(user_roles -> roles (role_id));
diesel::joinable!(user_roles -> users (user_id));
diesel::joinable!(users -> teams (team_id));
diesel::joinable!(users -> upload_profiles (default_upload_profile_id));

diesel::allow_tables_to_appear_in_same_query!(
    api_key_permissions,
    api_keys,
    base_images,
    conversion_profiles,
    output_images,
    projects,
    role_permissions,
    roles,
    sessions,
    storage_locations,
    teams,
    upload_profiles,
    user_roles,
    users,
);
