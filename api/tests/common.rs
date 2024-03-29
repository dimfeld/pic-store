use diesel::RunQueryDsl;
use eyre::Result;
use futures::Future;
use once_cell::sync::Lazy;

pub use crate::client::*;

use pic_store_api::Server;
use pic_store_db::object_id::{TeamId, UserId};
use pic_store_db::test::{create_database, DatabaseUser, TestDatabase};
use pic_store_db::users::NewUser;
use pic_store_db::PoolExt;
// use proc_macro::TokenStream;
// use quote::quote;

pub struct TestUser {
    pub team_id: TeamId,
    pub user_id: UserId,
    pub password: Option<String>,
    pub api_key: String,
    pub client: TestClient,
}

impl std::fmt::Debug for TestUser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TestUser")
            .field("team_id", &self.team_id)
            .field("user_id", &self.user_id)
            .field("password", &self.password)
            .field("api_key", &self.api_key)
            .finish_non_exhaustive()
    }
}

pub struct TestApp {
    pub database: TestDatabase,
    /// The ID of the precreated organization.
    pub team_id: TeamId,
    pub admin_user: TestUser,
    /// A client set to the base url of the server.
    pub client: TestClient,
    pub address: String,
    pub base_url: String,
}

async fn start_app(
    database: TestDatabase,
    team_id: TeamId,
    admin_user: DatabaseUser,
) -> Result<TestApp> {
    let queue_dir = temp_dir::TempDir::new().expect("Creating queue temp dir");
    let queue_path = queue_dir.path().join("queue.db");

    let config = pic_store_api::config::Config {
        database_url: database.url.clone(),
        port: 0, // Bind to random port
        host: "127.0.0.1".to_string(),
        queue_db_path: queue_path.to_string_lossy().to_string(),
        honeycomb_team: None,
        honeycomb_dataset: String::new(),
        env: "test".to_string(),
        jaeger_endpoint: None,
        allow_local_fs: true,
        cookie_key: "QjX+c1Nggom7lrxVTJFxMI7iQ0BRVr1oR9N64orRgdW3pp/SV+lE/1FOwo12UZj9QoBUUuv2rvcO0x+Omq+25Q==".to_string(),
        session_cookie_name: "sid".to_string(),
    };
    Lazy::force(&pic_store_test::TRACING);
    let server = pic_store_api::create_server(config).await?;
    let host = server.host.clone();
    let port = server.port;

    tokio::task::spawn(async move { server.run().await.unwrap() });

    let base_url = format!("http://{}:{}/api", host, port);
    let client = TestClient {
        base: base_url.clone(),
        client: reqwest::ClientBuilder::new()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Building client"),
    };

    let conn = database
        .pool
        .get()
        .await
        .expect("Getting postgres connection");

    let api_key = conn
        .interact(move |conn| {
            pic_store_api::api_key::make_key(conn, admin_user.user_id, false, None, None)
        })
        .await
        .unwrap()?
        .key;

    Ok(TestApp {
        database,
        team_id,
        admin_user: TestUser {
            team_id: admin_user.team_id,
            user_id: admin_user.user_id,
            password: admin_user.password,
            client: client.clone_with_api_key(api_key.clone()),
            api_key,
        },
        client,
        address: format!("{}:{}", host, port),
        base_url,
    })
}

pub async fn run_app_test<F, R>(f: F)
where
    F: FnOnce(TestApp) -> R,
    R: Future<Output = Result<(), eyre::Report>>,
{
    let (database, db_info) = create_database().await.expect("Creating database");
    let app = start_app(database.clone(), db_info.team_id, db_info.admin_user)
        .await
        .expect("Starting app");
    f(app).await.unwrap();
    database.drop_db().expect("Cleaning up");
}

impl TestApp {
    pub async fn add_team(&self, name: &str) -> Result<TeamId> {
        let team_id = TeamId::new();

        let team = pic_store_db::teams::NewTeam {
            id: team_id,
            name: name.to_string(),
        };

        self.database
            .pool
            .interact(move |conn| {
                diesel::insert_into(pic_store_db::teams::table)
                    .values(&team)
                    .execute(conn)?;
                Ok::<_, eyre::Report>(())
            })
            .await?;

        println!("Created team {}", team_id);
        Ok(team_id)
    }

    pub async fn add_user_with_password(
        &self,
        team_id: TeamId,
        name: &str,
        password: Option<&str>,
    ) -> Result<TestUser> {
        if password.is_some() {
            todo!("Password support will be implemented once the API supports creating users");
        }

        let hash = password
            .map(pic_store_auth::password::new_hash)
            .transpose()?;

        let user_id = UserId::new();
        let user = NewUser {
            id: user_id,
            name: name.to_string(),
            email: format!("test_user_{}@example.com", user_id),
            team_id,
            password_hash: hash,
            default_upload_profile_id: None,
        };

        let key = self
            .database
            .pool
            .interact(move |conn| {
                diesel::insert_into(pic_store_db::users::table)
                    .values(&user)
                    .execute(conn)?;

                let key = pic_store_api::api_key::make_key(conn, user_id, false, None, None)?;

                Ok::<_, eyre::Report>(key)
            })
            .await?
            .key;

        println!("Org {} added user {}: {}", team_id, name, user_id);
        Ok(TestUser {
            user_id,
            team_id,
            password: None,
            client: self.client.clone_with_api_key(key.clone()),
            api_key: key,
        })
    }

    pub async fn add_user(&self, team_id: TeamId, name: &str) -> Result<TestUser> {
        self.add_user_with_password(team_id, name, None).await
    }
}

/*
/** Compare hashmaps that have different value types, if those types implement PartialEq
 * on each other. */
#[macro_export]
macro_rules! compare_hashmaps {
    ($a: expr, $b: expr, $str: expr) => {
        if let Err(mismatch) = $crate::common::do_compare_hashmap(&$a, &$b) {
            panic!("{}\n{}", $str, mismatch);
        }
    };
    ($a: expr, $b: expr, $str: expr, $($fmt_args: expr),*) => {
        if let Err(mismatch) = $crate::common::do_compare_hashmap(&$a, &$b) {
            let msg = format!($str, $($fmt_args)*);
            panic!("{}\n{}", msg, mismatch);
        }
    };
}

pub fn do_compare_hashmap<K, A, B>(a: &FxHashMap<K, A>, b: &FxHashMap<K, B>) -> Result<(), String>
where
    K: std::hash::Hash + PartialEq + Eq + std::fmt::Debug,
    A: PartialEq<B> + std::fmt::Debug,
    B: std::fmt::Debug,
{
    if a.len() != b.len() {
        return Err(format!("{:?} was not equal to {:?}", a, b));
    }

    for (k, a_value) in a.iter() {
        let b_value = b.get(k).ok_or_else(|| format!(
            "key {:?} with value {:?} was present in left but not in right\nleft:  {:?}\nright: {:?}",
             k, a_value, a, b
        ))?;
        if a_value != b_value {
            return Err(format!(
                "key {:?} had  different value value\nleft:  {:?}\nright: {:?}",
                k, a_value, b_value
            ));
        }
    }

    Ok(())
}
*/
