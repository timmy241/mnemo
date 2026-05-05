use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "mnemo", about = "mnemo - an AI agent")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Interactive setup: configure model and WeChat login
    Setup,
}
