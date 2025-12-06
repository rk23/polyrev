use clap::Parser;
use tracing_subscriber::{fmt, EnvFilter};

mod cli;
mod config;
mod discovery;
mod error;
mod github;
mod output;
mod parser;
mod provider;
mod runner;
mod state;

use cli::{Cli, Commands};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize tracing
    let filter = if cli.verbose {
        EnvFilter::new("polyrev=debug")
    } else {
        EnvFilter::new("polyrev=info")
    };

    fmt().with_env_filter(filter).with_target(false).init();

    match cli.command {
        Commands::Run(args) => cli::run::execute(args).await,
        Commands::Issue(args) => cli::issue::execute(args).await,
        Commands::Init(args) => cli::init::execute(args),
        Commands::Schema => cli::schema::execute(),
    }
}
