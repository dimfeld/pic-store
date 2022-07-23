use clap::{Parser, Subcommand};

mod cmd;

#[derive(Parser, Debug)]
struct Args {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    #[clap(about = "Run the server")]
    Server(pic_store_api::config::Config),
    #[clap(about = "Administration and bootstrapping commands")]
    Admin(cmd::admin::AdminArgs),
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    color_eyre::install()?;
    dotenv::dotenv().ok();

    let cmd = Args::parse();
    match cmd.command {
        Commands::Server(config) => cmd::server::run(config).await?,
        Commands::Admin(cmd) => cmd::admin::admin_commands(cmd)?,
    };

    Ok(())
}
