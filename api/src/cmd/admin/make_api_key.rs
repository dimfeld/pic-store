use anyhow::Result;
use chrono::{DateTime, TimeZone, Utc};
use clap::Args;
use diesel::{prelude::*, Connection, PgConnection};
use pic_store_db::object_id::*;

use pic_store_api::api_key::make_key;

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

pub fn main(args: MakeApiKeyArgs) -> Result<()> {
    let mut conn = diesel::pg::PgConnection::establish(args.database.as_str())?;
    let key = make_key(
        &mut conn,
        args.user,
        args.no_inherit_user_permissions,
        args.description.as_deref(),
        args.expires,
    )?;

    println!("Key ID: {}", key.api_key_id);
    println!("Key: {}", key.key);

    Ok(())
}
