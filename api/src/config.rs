use clap::Parser;

#[derive(Debug, Parser)]
pub struct Config {
    #[clap(short, long, env, default_value_t = String::from("127.0.0.1"))]
    pub host: String,
    #[clap(short, long, env, default_value_t = 7205)]
    pub port: u16,

    #[clap(env, default_value_t = String::from("production"))]
    pub env: String,

    #[clap(long = "db", env)]
    pub database_url: String,

    #[clap(long, env)]
    pub honeycomb_team: Option<String>,
    #[clap(long, env, default_value_t = String::from("dev"))]
    pub honeycomb_dataset: String,

    #[clap(long, env, help = "A biscuit private key")]
    pub biscuit_key: String,
}
