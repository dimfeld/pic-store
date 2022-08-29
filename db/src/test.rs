use crate::object_id::{TeamId, UserId};
use crate::users::NewUser;
use crate::{Pool, PoolExt};
use anyhow::{anyhow, Result};
use deadpool_diesel::Manager;
use diesel::connection::SimpleConnection;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::Connection;
use diesel_migrations::{EmbeddedMigrations, MigrationHarness};
use futures::Future;
use lazy_static::lazy_static;
use std::str::FromStr;

#[derive(Clone)]
pub struct TestDatabase {
    pub name: String,
    pub pool: Pool,
    pub url: String,
    global_connect_str: String,
}

impl TestDatabase {
    pub fn drop_db(&self) -> Result<()> {
        let mut conn = PgConnection::establish(self.global_connect_str.as_str())?;
        diesel::sql_query(&format!(r##"DROP DATABASE "{}" (FORCE)"##, self.name))
            .execute(&mut conn)?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct DatabaseUser {
    pub team_id: TeamId,
    pub user_id: UserId,
    pub password: Option<String>,
}

fn escape(s: &str) -> String {
    s.replace('\\', r##"\\"##).replace('\'', r##"\'"##)
}

pub async fn run_database_test<F, R>(f: F)
where
    F: FnOnce(TestDatabase) -> R,
    R: Future<Output = Result<(), anyhow::Error>>,
{
    let (database, _, _) = create_database().await.expect("Creating database");
    f(database.clone()).await.unwrap();
    database.drop_db().expect("Cleaning up");
}

const MIGRATIONS: EmbeddedMigrations = diesel_migrations::embed_migrations!();

pub async fn create_database() -> Result<(TestDatabase, TeamId, DatabaseUser)> {
    dotenv::dotenv().ok();
    let host = std::env::var("TEST_DATABASE_HOST")
        .or_else(|_| std::env::var("DATABASE_HOST"))
        .unwrap_or_else(|_| "localhost".to_string());
    let port = std::env::var("TEST_DATABASE_PORT")
        .or_else(|_| std::env::var("DATABASE_PORT"))
        .map_err(anyhow::Error::new)
        .and_then(|val| val.parse::<u16>().map_err(|e| anyhow!(e)))
        .unwrap_or(5432);
    let user = std::env::var("TEST_DATABASE_USER").unwrap_or_else(|_| "postgres".to_string());
    let password = std::env::var("TEST_DATABASE_PASSWORD").unwrap_or_else(|_| "".to_string());

    let global_connect = format!("postgresql://{user}:{password}@{host}:{port}");
    let database = format!("pic_store_test_{}", crate::new_uuid().simple());
    println!("Database name: {}", database);

    let mut global_conn = PgConnection::establish(global_connect.as_str())?;

    diesel::sql_query(&format!(r##"CREATE DATABASE "{}""##, database)).execute(&mut global_conn)?;

    // The roles are global, but need to be set up. The migrations normally handle this but for
    // tests we need to make sure that the passwords are set.
    let roles_query = format!(
        r##"
DO $$BEGIN
  CREATE ROLE pic_store INHERIT LOGIN PASSWORD '{password}';
  EXCEPTION WHEN duplicate_object THEN NULL;
END; $$;
            "##,
        password = "test_db_password",
    );

    diesel::sql_query(roles_query.as_str()).execute(&mut global_conn)?;
    drop(global_conn);

    let db_conn_str = format!("{global_connect}/{database}");
    let manager = Manager::new(db_conn_str.clone(), deadpool_diesel::Runtime::Tokio1);
    let pool = Pool::builder(manager).max_size(4).build()?;

    let admin_user = pool
        .interact(|conn| {
            conn.run_pending_migrations(MIGRATIONS).unwrap();
            let admin_user = populate_database(conn)?;
            Ok::<_, anyhow::Error>(admin_user)
        })
        .await?;

    Ok((
        TestDatabase {
            pool,
            url: db_conn_str,
            name: database,
            global_connect_str: global_connect,
        },
        admin_user.team_id,
        admin_user,
    ))
}

pub const PASSWORD: &str = "test password";
const PASSWORD_HASH: &str = "$argon2id$v=19$m=15360,t=2,p=1$PUpyHXvHTSOKvr9Sc6vK8g$GSyd7TMMKrS7bkObHL3+aOtRmULRJTNP1xLP4C/3zzY";

lazy_static! {
    static ref ADMIN_USER_ID: UserId = std::env::var("ADMIN_USER_ID")
        .map(|u| UserId::from_str(u.as_str()).unwrap())
        .unwrap_or_else(|_| UserId::new());
}

fn populate_database(conn: &mut PgConnection) -> Result<DatabaseUser, anyhow::Error> {
    let user_id = *ADMIN_USER_ID;
    let team_id = TeamId::new();

    diesel::insert_into(crate::teams::table)
        .values(crate::teams::NewTeam {
            team_id,
            name: "Test Team".to_string(),
        })
        .execute(conn)?;

    diesel::insert_into(crate::users::table)
        .values(NewUser {
            user_id,
            team_id,
            name: "Test Admin User".to_string(),
            email: "user@example.com".to_string(),
            password_hash: Some(PASSWORD_HASH.to_string()),
        })
        .execute(conn)?;

    Ok(DatabaseUser {
        user_id,
        team_id,
        password: Some(PASSWORD.to_string()),
    })
}
