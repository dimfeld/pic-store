use clap::Parser;

#[derive(Debug, Parser)]
pub struct Config {
    #[clap(short, long, env, default_value_t = String::from("127.0.0.1"))]
    pub host: String,
    #[clap(short, long, env, default_value_t = 7205)]
    pub port: u16,

    #[clap(long = "db", env)]
    pub database_url: String,
}
