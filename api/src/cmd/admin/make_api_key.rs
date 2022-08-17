use anyhow::Result;
use chrono::{DateTime, TimeZone, Utc};
use clap::Args;
use diesel::{prelude::*, Connection, PgConnection};
use pic_store_db::object_id::*;

#[derive(Debug, Args)]
pub struct MakeApiKeyArgs {
    #[clap(short, long, help = "The user that owns the API key", env = "USER_ID")]
    user: UserId,
    #[clap(short, long, help = "Database connection string", env = "DATABASE_URL")]
    database: String,
    #[clap(short, long, help = "When the API key will expire")]
    expires: Option<DateTime<Utc>>,
    #[clap(short, long, help = "Key should not inherit user permissions")]
    no_inherit_user_permissions: bool,
    #[clap(name = "desc", long, help = "A description for the API key")]
    description: Option<String>,
}

pub fn make_key(
    conn: &mut PgConnection,
    user_id: UserId,
    no_inherit_user_permissions: bool,
    description: Option<&str>,
    expires: Option<DateTime<Utc>>,
) -> Result<String> {
    // Eventually all this code will be integrated into the ergo library itself.

    let default_date = Utc.ymd(3000, 1, 1).and_hms(0, 0, 0);
    let key = pic_store_auth::api_key::ApiKeyData::new(expires.unwrap_or(default_date));

    let user = pic_store_db::users::table
        .find(user_id)
        .first::<pic_store_db::users::User>(conn)?;

    let new_key = pic_store_db::api_keys::ApiKey {
        api_key_id: key.api_key_id,
        prefix: key.prefix,
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

    println!("Key ID: {}", key.api_key_id);
    println!("Key: {}", key.key);

    Ok(key.key)
}

pub fn main(args: MakeApiKeyArgs) -> Result<()> {
    let mut conn = diesel::pg::PgConnection::establish(args.database.as_str())?;
    make_key(
        &mut conn,
        args.user,
        args.no_inherit_user_permissions,
        args.description.as_deref(),
        args.expires,
    )?;
    Ok(())
}
