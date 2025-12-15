pub mod enqueue;
pub mod init;
pub mod issue;
pub mod plan;
pub mod postprocess;
pub mod run;
pub mod schema;
pub mod tui;

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

    /// Initialize polyrev: analyze repo, generate config and prompts
    Init(InitArgs),

    /// Run postprocessing on existing findings (dedupe/cluster)
    Postprocess(PostprocessArgs),

    /// Generate task DAG from a spec using parallel planning perspectives
    Plan(PlanArgs),

    /// Enqueue tasks from a plan to tandem
    Enqueue(EnqueueArgs),

    /// Launch interactive TUI for viewing findings and plans
    Tui(TuiArgs),

    /// Print JSON Schema for config validation
    Schema,
}

#[derive(Parser, Clone)]
pub struct PostprocessArgs {
    /// Config file
    #[arg(short, long, default_value = "polyrev.yaml")]
    pub config: PathBuf,

    /// Report directory containing findings
    #[arg(long)]
    pub report_dir: Option<PathBuf>,
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

    /// Create GitHub issues after run completes
    #[arg(long)]
    pub create_issues: bool,
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
    /// Directory to analyze (default: current directory)
    #[arg(long, default_value = ".")]
    pub target: PathBuf,

    /// Output path for generated config
    #[arg(long, default_value = "polyrev.yaml")]
    pub config: PathBuf,

    /// Output directory for generated prompts
    #[arg(long, default_value = "prompts")]
    pub prompts_dir: PathBuf,

    /// Number of reviewers to generate (1-6)
    #[arg(long, default_value = "3", value_parser = clap::value_parser!(u8).range(1..=6))]
    pub reviewers: u8,

    /// Provider to use for generation (claude_cli or codex_cli)
    #[arg(long, default_value = "claude_cli")]
    pub provider: String,

    /// Also create GitHub labels (requires --repo)
    #[arg(long)]
    pub labels: bool,

    /// Repository for label creation (owner/repo)
    #[arg(long)]
    pub repo: Option<String>,

    /// Preview without writing files
    #[arg(long)]
    pub dry_run: bool,

    /// Overwrite existing files
    #[arg(long)]
    pub force: bool,
}

#[derive(Parser, Clone)]
pub struct PlanArgs {
    /// The spec/feature to plan (can be multiple words)
    #[arg(value_name = "SPEC")]
    pub spec: Vec<String>,

    /// Read spec from file
    #[arg(long)]
    pub file: Option<PathBuf>,

    /// Fetch spec from GitHub issue number
    #[arg(long)]
    pub issue: Option<u64>,

    /// Config file
    #[arg(short, long, default_value = "polyrev.yaml")]
    pub config: PathBuf,

    /// Only run specific perspectives (comma-separated)
    #[arg(long, value_delimiter = ',')]
    pub perspectives: Option<Vec<String>>,

    /// Auto-select perspectives based on task (uses AI to choose)
    #[arg(long)]
    pub auto_select: bool,

    /// Maximum perspectives to run (with --auto-select, or limits --perspectives)
    #[arg(long, default_value = "4")]
    pub max_perspectives: usize,

    /// Preview without executing
    #[arg(long)]
    pub dry_run: bool,

    /// Skip the reduce step, output raw fragments
    #[arg(long)]
    pub skip_reduce: bool,

    /// Save intermediate fragments for debugging
    #[arg(long)]
    pub save_fragments: bool,

    /// Auto-approve without prompting
    #[arg(short, long)]
    pub yes: bool,

    /// Don't enqueue tasks to tandem after approval
    #[arg(long)]
    pub no_enqueue: bool,
}

#[derive(Parser, Clone)]
pub struct EnqueueArgs {
    /// Path to plan.json file
    #[arg(long, required = true)]
    pub plan: PathBuf,

    /// Config file (for Claude CLI binary path)
    #[arg(short, long, default_value = "polyrev.yaml")]
    pub config: PathBuf,

    /// Path to tandem SQLite database
    #[arg(long)]
    pub db: Option<PathBuf>,

    /// Prefix for task external IDs (default: plan filename)
    #[arg(long)]
    pub prefix: Option<String>,

    /// Preview without enqueuing
    #[arg(long)]
    pub dry_run: bool,

    /// Force enqueue even with unanswered questions or existing tasks
    #[arg(long)]
    pub force: bool,
}

#[derive(Parser, Clone)]
pub struct TuiArgs {
    /// Load findings from JSON file
    #[arg(long)]
    pub findings: Option<PathBuf>,

    /// Load plan from JSON file
    #[arg(long)]
    pub plan: Option<PathBuf>,

    /// Reports directory to scan for findings
    #[arg(long, default_value = "reports")]
    pub report_dir: PathBuf,

    /// Start in plan view (vs findings view)
    #[arg(long)]
    pub plan_mode: bool,
}
