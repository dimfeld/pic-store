use async_trait::async_trait;
use chrono::Utc;
use diesel::Queryable;
use uuid::Uuid;

use db::object_id::{TeamId, UserId};

use pic_store_auth as auth;
use pic_store_db as db;

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
        key_id: &Uuid,
        hash: &auth::api_key::Hash,
    ) -> Result<Self::FetchData, Self::Error> {
        let conn = self.db.get().await?;
        conn.interact(move |conn| {
            db::api_keys::table
                .filter(db::api_keys::api_key_id.eq(key_id))
                .filter(db::api_keys::hash.eq(hash.as_bytes()))
                .filter(db::api_keys::expires.gt(diesel::dsl::now))
                .select((
                    db::api_keys::api_key_id,
                    db::api_keys::user_id,
                    db::api_keys::team_id,
                    db::api_keys::inherits_user_permissions,
                ))
                .first::<ApiKeyData>(conn)
        })
        .await?
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
            hash: key.hash.as_bytes(),
            team_id: data.team_id,
            user_id: data.user_id,
            inherits_user_permissions: data.inherits_user_permissions,
            expires: key.expires,
            created: Utc::now(),
        };

        let conn = self.db.get().await?;
        conn.interact(move |conn| diesel::insert_into(&input).execute(conn))??;
        Ok(())
    }

    async fn disable_api_key(&self, key_id: &Uuid) -> Result<(), Self::Error> {
        let conn = self.db.get().await?;
        conn.interact(move |conn| {
            diesel::delete(db::api_keys::table).filter(db::api_keys::api_key_id.eq(key_id))
        })??;
        Ok(())
    }

    fn api_key_prefix(&self) -> &'static str {
        "ps1"
    }
}
