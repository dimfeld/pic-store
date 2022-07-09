use async_trait::async_trait;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use ulid::Ulid;
use uuid::Uuid;

use db::object_id::{TeamId, UserId};

use pic_store_auth as auth;
use pic_store_db as db;

use crate::Error;

#[derive(Queryable)]
pub struct ApiKeyData {
    pub api_key_id: Uuid,
    pub user_id: UserId,
    pub team_id: TeamId,
    pub inherits_user_permissions: bool,
}

pub struct ApiKeyNewData {
    pub team_id: TeamId,
    pub user_id: UserId,
    pub name: String,
    pub inherits_user_permissions: bool,
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
        let conn = self.db.get().await?;
        let info = conn
            .interact(move |conn| {
                db::api_keys::table
                    .filter(db::api_keys::api_key_id.eq(key_id))
                    .filter(db::api_keys::hash.eq(hash.as_bytes().as_slice()))
                    .filter(db::api_keys::expires.gt(diesel::dsl::now))
                    .select((
                        db::api_keys::api_key_id,
                        db::api_keys::user_id,
                        db::api_keys::team_id,
                        db::api_keys::inherits_user_permissions,
                    ))
                    .first::<ApiKeyData>(conn)
            })
            .await??;

        Ok(info)
    }

    async fn create_api_key(
        &self,
        key: auth::api_key::ApiKeyData,
        data: ApiKeyNewData,
    ) -> Result<(), Self::Error> {
        let input = db::api_keys::ApiKey {
            api_key_id: key.api_key_id,
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
                .filter(db::api_keys::api_key_id.eq(key_id))
                .execute(conn)
        })
        .await??;

        Ok(())
    }

    fn api_key_prefix(&self) -> &'static str {
        "ps1"
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
                    session_id: Ulid::new().into(),
                    user_id,
                    expires,
                };

                diesel::insert_into(db::sessions::table)
                    .values(&input)
                    .execute(conn)?;

                Ok::<Uuid, crate::Error>(input.session_id)
            })
            .await??;

        Ok(session.to_string())
    }

    async fn get_session(&self, id: &str) -> Result<Self::SessionFetchData, Self::Error> {
        let session_id = id.parse::<Uuid>().map_err(|_| Error::InvalidSessionId)?;
        let conn = self.db.get().await?;
        conn.interact(move |conn| {
            db::sessions::table
                .inner_join(db::users::table)
                .filter(db::sessions::session_id.eq(session_id))
                .filter(db::sessions::expires.gt(diesel::dsl::now))
                .select((db::sessions::user_id, db::users::team_id))
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
                .filter(db::sessions::session_id.eq(session_id))
                .execute(conn)
        })
        .await??;

        Ok(())
    }
}
