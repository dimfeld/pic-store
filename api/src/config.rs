use clap::Parser;

#[derive(Debug, Parser)]
pub struct Config {
    #[clap(long, env, default_value_t = String::from("127.0.0.1"))]
    pub host: String,
    #[clap(long, env, default_value_t = 7205)]
    pub port: u16,

    #[clap(long, env, default_value_t = String::from("queue.db"))]
    pub queue_db_path: String,

    #[clap(env, default_value_t = String::from("production"))]
    pub env: String,

    #[clap(long = "db", env)]
    pub database_url: String,

    #[clap(long, env)]
    pub honeycomb_team: Option<String>,
    #[clap(long, env, default_value_t = String::from("dev"))]
    pub honeycomb_dataset: String,

    #[clap(long, env)]
    pub jaeger_endpoint: Option<String>,

    #[clap(
        long,
        env,
        help = "A key for signed cookies, base64 at least 88 characters (64 bytes)"
    )]
    pub cookie_key: String,

    #[clap(long, env, help = "The name of the session cookie", default_value_t = String::from("sid"))]
    pub session_cookie_name: String,

    #[clap(
        long,
        env,
        help = "Allow using the local filesystem as an upload destination",
        default_value_t = false
    )]
    pub allow_local_fs: bool,
}
