use async_trait::async_trait;
use auth::session::{SessionCookieManager, SessionManager};
use auth::{AuthenticationLayer, RequestUser};
use axum::extract::FromRequestParts;
use chrono::{DateTime, Utc};
use db::PoolExt;
use diesel::dsl::sql;
use diesel::prelude::*;
use http::request::Parts;
use serde::Deserialize;
use ulid::Ulid;
use uuid::Uuid;

use db::object_id::{ProjectId, RoleId, TeamId, UploadProfileId, UserId};

use pic_store_auth as auth;
use pic_store_db as db;

use crate::Error;

pub const API_KEY_PREFIX: &str = "ps1";

#[derive(Queryable)]
pub struct ApiKeyData {
    pub api_key_id: Uuid,
    pub user_id: UserId,
    pub team_id: TeamId,
    pub roles: Vec<RoleId>,
    pub inherits_user_permissions: bool,
    pub default_upload_profile_id: Option<UploadProfileId>,
}

pub struct ApiKeyNewData {
    pub team_id: TeamId,
    pub user_id: UserId,
    pub name: String,
    pub inherits_user_permissions: bool,
    pub default_upload_profile_id: Option<UploadProfileId>,
}

#[derive(Clone)]
pub struct ApiKeyStore {
    pub db: db::Pool,
}

#[async_trait]
impl auth::api_key::ApiKeyStore for ApiKeyStore {
    type FetchData = ApiKeyData;
    type NewData = ApiKeyNewData;
    type Error = crate::Error;

    async fn lookup_api_key(
        &self,
        key_id: Uuid,
        hash: auth::api_key::Hash,
    ) -> Result<Self::FetchData, Self::Error> {
        #[derive(Queryable)]
        struct ApiKeyLookupResult {
            pub api_key_id: Uuid,
            pub user_id: UserId,
            pub team_id: TeamId,
            pub roles: Vec<RoleId>,
            pub inherits_user_permissions: bool,
            pub api_key_default_upload_profile_id: Option<UploadProfileId>,
            pub user_default_upload_profile_id: Option<UploadProfileId>,
        }

        let info = self.db
            .interact(move |conn| {
                db::api_keys::table
                    .left_join(
                        db::user_roles::table.on(db::user_roles::user_id.eq(db::api_keys::user_id)),
                    )
                    .group_by( db::api_keys::id)
                    .filter(db::api_keys::id.eq(key_id))
                    .filter(db::api_keys::hash.eq(hash.as_bytes().as_slice()))
                    .filter(db::api_keys::expires.gt(diesel::dsl::now))
                    .select((
                        db::api_keys::id,
                        db::api_keys::user_id,
                        db::api_keys::team_id,
                        sql::<diesel::sql_types::Array<diesel::sql_types::Uuid>>(
                            "COALESCE(ARRAY_AGG(role_id) FILTER (WHERE role_id IS NOT NULL), '{}') AS roles",
                        ),
                        db::bool_or(db::api_keys::inherits_user_permissions),
                        db::api_keys::default_upload_profile_id,
                        db::users::table
                            .select(db::users::default_upload_profile_id)
                            .filter(db::users::id.eq(db::api_keys::user_id))
                            .single_value(),
                    ))
                    .first::<ApiKeyLookupResult>(conn)
                    .optional()
                    .map_err(Error::from)
            })
            .await?;

        let info = info.ok_or(crate::Error::ApiKeyNotFound)?;

        Ok(ApiKeyData {
            api_key_id: info.api_key_id,
            user_id: info.user_id,
            team_id: info.team_id,
            roles: info.roles,
            inherits_user_permissions: info.inherits_user_permissions,
            default_upload_profile_id: info
                .api_key_default_upload_profile_id
                .or(info.user_default_upload_profile_id),
        })
    }

    async fn create_api_key(
        &self,
        key: auth::api_key::ApiKeyData,
        data: ApiKeyNewData,
    ) -> Result<(), Self::Error> {
        let input = db::api_keys::ApiKey {
            id: key.id,
            name: data.name,
            prefix: key.prefix,
            hash: key.hash.as_bytes().to_vec(),
            team_id: data.team_id,
            user_id: data.user_id,
            inherits_user_permissions: data.inherits_user_permissions,
            expires: key.expires,
            created: Utc::now(),
        };

        let conn = self.db.get().await?;
        conn.interact(move |conn| {
            diesel::insert_into(db::api_keys::table)
                .values(&input)
                .execute(conn)
        })
        .await??;
        Ok(())
    }

    async fn disable_api_key(&self, key_id: Uuid) -> Result<(), Self::Error> {
        let conn = self.db.get().await?;

        conn.interact(move |conn| {
            diesel::delete(db::api_keys::table)
                .filter(db::api_keys::id.eq(key_id))
                .execute(conn)
        })
        .await??;

        Ok(())
    }

    fn api_key_prefix(&self) -> &'static str {
        API_KEY_PREFIX
    }
}

#[derive(Clone)]
pub struct SessionStore {
    pub db: db::Pool,
}

#[derive(Queryable)]
pub struct SessionData {
    user_id: UserId,
    team_id: TeamId,
    roles: Vec<RoleId>,
    default_upload_profile_id: Option<UploadProfileId>,
}

#[async_trait]
impl auth::session::SessionStore for SessionStore {
    type UserId = UserId;
    type SessionFetchData = SessionData;
    type Error = crate::Error;

    async fn create_session(
        &self,
        user_id: UserId,
        expires: DateTime<Utc>,
    ) -> Result<String, Self::Error> {
        let conn = self.db.get().await?;
        let session = conn
            .interact(move |conn| {
                let input = db::sessions::Session {
                    id: Ulid::new().into(),
                    user_id,
                    expires,
                };

                diesel::insert_into(db::sessions::table)
                    .values(&input)
                    .execute(conn)?;

                Ok::<Uuid, crate::Error>(input.id)
            })
            .await??;

        Ok(session.to_string())
    }

    async fn get_session(&self, id: &str) -> Result<Self::SessionFetchData, Self::Error> {
        let session_id = id.parse::<Uuid>().map_err(|_| Error::InvalidSessionId)?;
        let conn = self.db.get().await?;
        conn.interact(move |conn| {
            db::sessions::table
                .inner_join(db::users::table.inner_join(db::user_roles::table))
                .group_by(db::users::id)
                .filter(db::sessions::id.eq(session_id))
                .filter(db::sessions::expires.gt(diesel::dsl::now))
                .select((
                    db::users::id,
                    db::users::team_id,
                    db::array_agg(db::user_roles::role_id),
                    db::users::default_upload_profile_id,
                ))
                .first::<SessionData>(conn)
        })
        .await?
        .map_err(Error::from)
    }

    async fn delete_session(&self, id: &str) -> Result<(), Self::Error> {
        let session_id = id.parse::<Uuid>().map_err(|_| Error::InvalidSessionId)?;
        let conn = self.db.get().await?;
        conn.interact(move |conn| {
            diesel::delete(db::sessions::table)
                .filter(db::sessions::id.eq(session_id))
                .execute(conn)
        })
        .await??;

        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct UserInfo {
    pub user_id: UserId,
    pub team_id: TeamId,
    pub roles: Vec<RoleId>,
    pub default_upload_profile_id: Option<UploadProfileId>,
}

impl From<RequestUser<ApiKeyData, SessionData>> for UserInfo {
    fn from(u: RequestUser<ApiKeyData, SessionData>) -> Self {
        match u {
            RequestUser::ApiKey(key) => UserInfo {
                user_id: key.user_id,
                team_id: key.team_id,
                roles: key.roles,
                default_upload_profile_id: key.default_upload_profile_id,
            },
            RequestUser::Session(s) => UserInfo {
                user_id: s.user_id,
                team_id: s.team_id,
                roles: s.roles,
                default_upload_profile_id: s.default_upload_profile_id,
            },
        }
    }
}

pub fn auth_layer(
    db: db::Pool,
    cookie_name: String,
    cookie_key_b64: &str,
) -> AuthenticationLayer<UserInfo, ApiKeyStore, SessionStore> {
    let api_store = ApiKeyStore { db: db.clone() };

    let session_store = SessionStore { db };

    let cookie_key = tower_cookies::Key::from(
        &base64::decode(cookie_key_b64).expect("cookie_key must be base64"),
    );

    let session_manager = SessionManager {
        store: session_store,
        cookies: SessionCookieManager {
            signing_key: cookie_key,
            cookie_name,
        },
        expire_days: 36500,
    };

    AuthenticationLayer::new(api_store, session_manager)
}

pub struct Authenticated(pub UserInfo);

#[async_trait]
impl<S> FromRequestParts<S> for Authenticated
where
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request_parts(req: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        req.extensions
            .get::<UserInfo>()
            .cloned()
            .map(Self)
            .ok_or(Error::Unauthenticated)
    }
}

pub fn must_have_permission_on_project(
    conn: &mut PgConnection,
    user: &UserInfo,
    project_id: ProjectId,
    permission: db::permissions::ProjectPermission,
) -> Result<(), crate::Error> {
    if db::permissions::has_permission_on_project(
        conn,
        user.team_id,
        &user.roles,
        Some(project_id),
        permission,
    )? {
        Ok(())
    } else {
        Err(Error::MissingPermission(permission.into()))
    }
}
