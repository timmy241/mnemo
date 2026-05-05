use clap::Parser;

mod cli;
mod config;
mod db;
mod llm;
mod run;
mod setup;

use cli::{Cli, Command};
use config::AppConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Command::Setup) => setup::run().await,
        None => {
            let config = AppConfig::load()?;
            run::run(config).await
        }
    }
}
