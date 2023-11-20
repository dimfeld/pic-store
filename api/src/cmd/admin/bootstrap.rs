use std::{collections::HashMap, env, path::Path};

use base64::Engine;
use chrono::{DateTime, Utc};
use clap::Args;
use db::object_id::*;
use diesel::{prelude::*, sql_query};
use eyre::{eyre, Result};
use pic_store_api::auth::API_KEY_PREFIX;
use pic_store_db as db;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Args)]
pub struct BootstrapArgs {
    /// A directory containing JSON files to load
    #[clap(env="BOOTSTRAP_LOCATION", default_value_t = String::from("./bootstrap_data"))]
    location: String,
}

pub fn bootstrap(args: BootstrapArgs) -> Result<(), eyre::Report> {
    let database_url = env::var("DATABASE_URL")?;
    let mut conn = PgConnection::establish(database_url.as_str())?;

    let file_glob = format!("{}/**/*.json", args.location);

    let vars = liquid::to_object(&env::vars().collect::<HashMap<_, _>>())?;

    let parser = liquid::ParserBuilder::with_stdlib().build()?;

    conn.build_transaction().run(move |conn| {
        // Set constraints deferrable so that we can load the objects without having to sort them
        // topologically by foreign key.
        sql_query("SET CONSTRAINTS ALL DEFERRED").execute(conn)?;
        for file in glob::glob(file_glob.as_str())? {
            let file = file?;
            apply_file(conn, &parser, &vars, &file)?;
        }

        Ok::<_, eyre::Report>(())
    })?;

    Ok(())
}

fn apply_file(
    conn: &mut PgConnection,
    parser: &liquid::Parser,
    vars: &liquid::Object,
    filename: &Path,
) -> Result<(), eyre::Report> {
    println!("Applying {}", filename.display());

    let template = parser.parse_file(filename)?;
    let rendered = template.render(vars)?;
    let objs: serde_json::Value = serde_json::from_str(rendered.as_str())?;

    let final_path = filename.file_name().unwrap().to_string_lossy();

    match objs {
        serde_json::Value::Array(a) => {
            for obj in a {
                if let serde_json::Value::Object(_) = &obj {
                    apply_object(conn, final_path.as_ref(), obj)?;
                } else {
                    return Err(eyre!("Expected object, found {obj:?}"));
                }
            }
        }
        objs @ serde_json::Value::Object(_) => apply_object(conn, final_path.as_ref(), objs)?,
        _ => return Err(eyre!("Expected object, found {objs:?}")),
    }

    Ok(())
}

macro_rules! insert_object {
    ($table: expr, $type: ty, $conn: expr, $obj: expr) => {{
        let value: $type = serde_json::from_value($obj)?;
        diesel::insert_into($table).values(&value).execute($conn)?;
    }};
}

#[derive(Deserialize)]
pub struct ApiKeyInput {
    key: String,
    name: String,
    team_id: TeamId,
    user_id: UserId,
    inherits_user_permissions: bool,
    expires: DateTime<Utc>,
}

fn apply_object(
    conn: &mut PgConnection,
    filename: &str,
    obj: serde_json::Value,
) -> eyre::Result<()> {
    let object_type = filename
        .rsplit('.')
        .nth(1)
        .ok_or_else(|| eyre!("No object type found in filename {filename:?}"))?;

    match object_type {
        "user" | "users" => insert_object!(db::users::table, db::users::NewUser, conn, obj),
        "user_role" | "user_roles" => insert_object!(
            db::user_roles::table,
            db::user_roles::UserAndRole,
            conn,
            obj
        ),
        "team" | "teams" => insert_object!(db::teams::table, db::teams::NewTeam, conn, obj),
        "project" | "projects" => {
            insert_object!(db::projects::table, db::projects::NewProject, conn, obj)
        }
        "conversion_profile" | "conversion_profiles" => insert_object!(
            db::conversion_profiles::table,
            db::conversion_profiles::NewConversionProfile,
            conn,
            obj
        ),
        "storage_location" | "storage_locations" => insert_object!(
            db::storage_locations::table,
            db::storage_locations::NewStorageLocation,
            conn,
            obj
        ),
        "upload_profile" | "upload_profiles" => insert_object!(
            db::upload_profiles::table,
            db::upload_profiles::NewUploadProfile,
            conn,
            obj
        ),
        "role" | "roles" => insert_object!(db::roles::table, db::roles::NewRole, conn, obj),
        "role_permission" | "role_permissions" => insert_object!(
            db::role_permissions::table,
            db::role_permissions::RolePermission,
            conn,
            obj
        ),
        "api_key" | "api_keys" => {
            let input: ApiKeyInput = serde_json::from_value(obj)?;

            // Parse the key into its component parts, so we can recreate it.
            let parts = input.key.split('.').collect::<Vec<_>>();
            if parts.len() != 3 {
                return Err(eyre!("API key must have 3 parts"));
            }

            if parts[0] != API_KEY_PREFIX {
                return Err(eyre!("API KEY must start with {API_KEY_PREFIX}."));
            }

            let engine = base64::engine::general_purpose::URL_SAFE_NO_PAD;
            let id_data = engine.decode(parts[1])?;
            let id = Uuid::from_slice(&id_data)?;
            let random_data = engine.decode(parts[2])?;
            let random = Uuid::from_slice(&random_data)?;

            let data = pic_store_auth::api_key::ApiKeyData::from_params(
                API_KEY_PREFIX,
                id,
                random,
                input.expires,
            );

            let value = db::api_keys::ApiKey {
                id,
                name: input.name,
                prefix: data.prefix,
                hash: data.hash.as_bytes().to_vec(),
                team_id: input.team_id,
                user_id: input.user_id,
                inherits_user_permissions: input.inherits_user_permissions,
                created: Utc::now(),
                expires: input.expires,
            };

            diesel::insert_into(db::api_keys::table)
                .values(&value)
                .execute(conn)?;
        }
        _ => return Err(eyre!("Unknown object type in filename {filename:?}")),
    };

    Ok(())
}
