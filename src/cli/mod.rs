pub mod init;
pub mod issue;
pub mod run;
pub mod schema;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "polyrev")]
#[command(
    author,
    version,
    about = "Parallel code review orchestrator for Claude Code and Codex CLI"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Enable verbose/debug logging
    #[arg(short, long, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Execute reviewers and produce reports
    Run(RunArgs),

    /// Create GitHub issues from reports
    Issue(IssueArgs),

    /// Initialize GitHub repo with required labels
    Init(InitArgs),

    /// Print JSON Schema for config validation
    Schema,
}

#[derive(Parser, Clone)]
pub struct RunArgs {
    /// Path to config file
    #[arg(short, long, default_value = "polyrev.yaml")]
    pub config: PathBuf,

    /// Override max parallel reviewers
    #[arg(long)]
    pub concurrency: Option<usize>,

    /// Override output directory
    #[arg(long)]
    pub report_dir: Option<PathBuf>,

    /// Only review changed files since this ref (e.g., main, HEAD~5)
    #[arg(long)]
    pub diff_base: Option<String>,

    /// Run specific reviewers only (comma-separated)
    #[arg(long, value_delimiter = ',')]
    pub reviewers: Option<Vec<String>>,

    /// Run specific scopes only (comma-separated)
    #[arg(long, value_delimiter = ',')]
    pub scopes: Option<Vec<String>>,

    /// Show plan without executing
    #[arg(long)]
    pub dry_run: bool,

    /// Exit 1 if any p0 findings (CI mode)
    #[arg(long)]
    pub fail_on_critical: bool,

    /// Force re-run even if reviewer already ran today
    #[arg(long)]
    pub force: bool,
}

#[derive(Parser, Clone)]
pub struct IssueArgs {
    /// Specific .findings.json files to upload (or scan --report-dir if none given)
    #[arg(value_name = "FILE")]
    pub files: Vec<PathBuf>,

    /// Reports directory to scan (default: reports/)
    #[arg(long, default_value = "reports")]
    pub report_dir: PathBuf,

    /// Config file (for GitHub settings)
    #[arg(long, default_value = "polyrev.yaml")]
    pub config: PathBuf,

    /// Preview issues without creating
    #[arg(long)]
    pub dry_run: bool,

    /// Ignore dedupe, create all issues
    #[arg(long)]
    pub force: bool,

    /// Override repository (owner/repo)
    #[arg(long)]
    pub repo: Option<String>,
}

#[derive(Parser, Clone)]
pub struct InitArgs {
    /// Repository to initialize (owner/repo)
    #[arg(long)]
    pub repo: Option<String>,

    /// Config file (for default repo)
    #[arg(long, default_value = "polyrev.yaml")]
    pub config: PathBuf,

    /// Preview labels without creating
    #[arg(long)]
    pub dry_run: bool,
}
