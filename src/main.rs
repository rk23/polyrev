use clap::Parser;
use tracing_subscriber::{fmt, EnvFilter};

mod cli;
mod config;
mod discovery;
mod error;
mod github;
mod output;
mod parser;
mod planner;
mod postprocess;
mod provider;
mod runner;
mod state;
mod tui;

use cli::{Cli, Commands};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize tracing - only show logs with --verbose
    let filter = if cli.verbose {
        EnvFilter::new("polyrev=debug")
    } else {
        EnvFilter::new("polyrev=warn")
    };

    fmt().with_env_filter(filter).with_target(false).init();

    match cli.command {
        Commands::Run(args) => cli::run::execute(args).await,
        Commands::Issue(args) => cli::issue::execute(args).await,
        Commands::Init(args) => cli::init::execute(args),
        Commands::Postprocess(args) => cli::postprocess::execute(args).await,
        Commands::Plan(args) => cli::plan::execute(args).await,
        Commands::Enqueue(args) => cli::enqueue::execute(args),
        Commands::Tui(args) => cli::tui::execute(args),
        Commands::Schema => cli::schema::execute(),
    }
}
