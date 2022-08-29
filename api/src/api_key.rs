use anyhow::Result;
use chrono::{DateTime, TimeZone, Utc};
use diesel::{prelude::*, Connection, PgConnection};
use pic_store_auth::api_key::ApiKeyData;
use pic_store_db::object_id::UserId;

pub fn make_key(
    conn: &mut PgConnection,
    user_id: UserId,
    no_inherit_user_permissions: bool,
    description: Option<&str>,
    expires: Option<DateTime<Utc>>,
) -> Result<ApiKeyData> {
    // Eventually all this code will be integrated into the ergo library itself.

    let default_date = Utc.ymd(3000, 1, 1).and_hms(0, 0, 0);
    let key = ApiKeyData::new(expires.unwrap_or(default_date));

    let user = pic_store_db::users::table
        .find(user_id)
        .first::<pic_store_db::users::User>(conn)?;

    let new_key = pic_store_db::api_keys::ApiKey {
        api_key_id: key.api_key_id,
        prefix: key.prefix.clone(),
        hash: key.hash.as_bytes().to_vec(),
        team_id: user.team_id,
        user_id,
        name: description.unwrap_or("").to_string(),
        inherits_user_permissions: !no_inherit_user_permissions,
        expires: key.expires,
        created: Utc::now(),
    };

    diesel::insert_into(pic_store_db::api_keys::table)
        .values(&new_key)
        .execute(conn)?;

    Ok(key)
}
