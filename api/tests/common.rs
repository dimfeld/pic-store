use anyhow::Result;
use diesel::RunQueryDsl;
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
    let config = pic_store_api::config::Config {
        database_url: database.url.clone(),
        port: 0, // Bind to random port
        host: "127.0.0.1".to_string(),
        honeycomb_team: None,
        honeycomb_dataset: String::new(),
        env: "test".to_string(),
        jaeger_endpoint: None,
        allow_local_fs: true,
        cookie_key: "QjX+c1Nggom7lrxVTJFxMI7iQ0BRVr1oR9N64orRgdW3pp/SV+lE/1FOwo12UZj9QoBUUuv2rvcO0x+Omq+25Q==".to_string(),
        session_cookie_name: "sid".to_string(),
    };
    Lazy::force(&pic_store_test::TRACING);
    let Server { server, host, port } = pic_store_api::run_server(config).await?;

    tokio::task::spawn(async move {
        let server_err = server.await;
        server_err
        // let shutdown_err = shutdown.shutdown().await;
        // match (server_err, shutdown_err) {
        //     (Err(e), _) => Err(anyhow!(e)),
        //     (Ok(_), Err(e)) => Err(anyhow!(e)),
        //     (Ok(_), Ok(_)) => Ok(()),
        // }
    });

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
    R: Future<Output = Result<(), anyhow::Error>>,
{
    let (database, team_id, admin_user) = create_database().await.expect("Creating database");
    let app = start_app(database.clone(), team_id, admin_user)
        .await
        .expect("Starting app");
    f(app).await.unwrap();
    database.drop_db().expect("Cleaning up");
}

impl TestApp {
    pub async fn add_team(&self, name: &str) -> Result<TeamId> {
        let team_id = TeamId::new();

        let team = pic_store_db::teams::NewTeam {
            team_id,
            name: name.to_string(),
        };

        self.database
            .pool
            .interact(move |conn| {
                diesel::insert_into(pic_store_db::teams::table)
                    .values(&team)
                    .execute(conn)?;
                Ok::<_, anyhow::Error>(())
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
            user_id,
            name: name.to_string(),
            email: format!("test_user_{}@example.com", user_id),
            team_id,
            password_hash: hash,
        };

        let key = self
            .database
            .pool
            .interact(move |conn| {
                diesel::insert_into(pic_store_db::users::table)
                    .values(&user)
                    .execute(conn)?;

                let key = pic_store_api::api_key::make_key(conn, user_id, false, None, None)?;

                Ok::<_, anyhow::Error>(key)
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

// #[proc_macro_attribute]
// pub fn app_test(_: TokenStream, item: TokenStream) -> TokenStream {
//     let mut input = syn::parse_macro_input!(item as syn::ItemFn);
//     let attrs = &input.attrs;
//     let vis = &input.vis;
//     let sig = &mut input.sig;
//     let body = &input.block;
//     let mut has_test_attr = false;
//
//     for attr in attrs {
//         if attr.path.is_ident("test") {
//             has_test_attr = true;
//         }
//     }
//
//     if sig.asyncness.is_none() {
//         return syn::Error::new_spanned(
//             input.sig.fn_token,
//             "the async keyword is missing from the function declaration",
//         )
//         .to_compile_error()
//         .into();
//     }
//
//     sig.asyncness = None;
//
//     let missing_test_attr = if has_test_attr {
//         quote!()
//     } else {
//         quote!(#[test])
//     };
//
//     let appname = match sig.inputs.first() {
//         Some(syn::FnArg::Typed(syn::PatType { pat: p, .. })) => match &**p {
//             syn::Pat::Ident(p) => p.clone(),
//             _ => {
//                 return syn::Error::new_spanned(
//                     input.sig.fn_token,
//                     "first argument must be a TestApp",
//                 )
//                 .to_compile_error()
//                 .into()
//             }
//         },
//         _ => {
//             return syn::Error::new_spanned(input.sig.fn_token, "first argument must be a TestApp")
//                 .to_compile_error()
//                 .into();
//         }
//     };
//
//     // Remove all the arguments.
//     sig.inputs = syn::punctuated::Punctuated::new();
//
//     (quote! {
//         #missing_test_attr
//         #(#attrs)*
//         #vis #sig {
//             actix_rt::System::new()
//                 .block_on(async {
//                     let #appname = {
//                         let (database, team__id, admin_user) = ergo_database::create_database().await.expect("Creating database");
//                         crate::common::start_app(database, team__id, admin_user)
//                             .await
//                             .expect("Starting app");
//                     };
//
//                     #body
//                 })
//         }
//     })
//     .into()
// }
