use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use super::defaults::*;
use crate::planner::Perspective;

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct Config {
    #[serde(default = "default_version")]
    pub version: u32,

    #[serde(default = "default_target")]
    pub target: PathBuf,

    #[serde(default = "default_concurrency")]
    pub concurrency: usize,

    #[serde(default = "default_report_dir")]
    pub report_dir: PathBuf,

    #[serde(default)]
    pub dry_run: bool,

    #[serde(default)]
    pub diff_base: Option<String>,

    #[serde(default)]
    pub github: GithubConfig,

    #[serde(default)]
    pub providers: ProvidersConfig,

    #[serde(default)]
    pub retry: RetryConfig,

    #[serde(default)]
    pub postprocess: PostProcessConfig,

    #[serde(default)]
    pub planning: Option<PlanningConfig>,

    #[serde(default = "default_timeout_sec")]
    pub timeout_sec: u64,

    #[serde(default = "default_max_files")]
    pub max_files: usize,

    #[serde(default = "default_launch_delay_ms")]
    pub launch_delay_ms: u64,

    #[serde(default)]
    pub scopes: HashMap<String, Scope>,

    #[serde(default)]
    pub reviewers: Vec<Reviewer>,
}

/// Configuration for parallel planning perspectives
#[derive(Debug, Clone, Default, Deserialize, Serialize, JsonSchema)]
pub struct PlanningConfig {
    /// Planning perspectives to run
    #[serde(default)]
    pub perspectives: Vec<Perspective>,

    /// Path to the reducer prompt
    #[serde(default)]
    pub reducer_prompt: Option<PathBuf>,

    /// Require human approval before enqueueing
    #[serde(default = "default_true")]
    pub require_human_approval: bool,

    /// What to do with unresolved questions: "block", "ask", "proceed_with_defaults"
    #[serde(default = "default_on_unresolved")]
    pub on_unresolved_questions: String,
}

fn default_on_unresolved() -> String {
    "block".to_string()
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, JsonSchema)]
pub struct GithubConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,

    #[serde(default)]
    pub repo: Option<String>,

    #[serde(default)]
    pub labels: Vec<String>,

    #[serde(default)]
    pub assignees: Vec<String>,

    #[serde(default = "default_true")]
    pub dedupe: bool,

    #[serde(default)]
    pub dedupe_action: DedupeAction,

    /// Auto-trigger an AI agent to fix created issues
    #[serde(default)]
    pub auto_fix: AutoFixConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct AutoFixConfig {
    /// Enable automatic agent triggering on new issues
    #[serde(default = "default_false")]
    pub enabled: bool,

    /// Which agent to trigger: "claude" or "codex"
    #[serde(default = "default_auto_fix_agent")]
    pub agent: String,

    /// Custom prompt to include in the comment
    #[serde(default = "default_auto_fix_prompt")]
    pub prompt: String,
}

impl Default for AutoFixConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            agent: default_auto_fix_agent(),
            prompt: default_auto_fix_prompt(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum DedupeAction {
    #[default]
    Skip,
    Comment,
    Reopen,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, JsonSchema)]
pub struct ProvidersConfig {
    #[serde(default)]
    pub claude_cli: ClaudeCliConfig,

    #[serde(default)]
    pub codex_cli: CodexCliConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct ClaudeCliConfig {
    #[serde(default = "default_claude_binary")]
    pub binary: PathBuf,

    #[serde(default = "default_claude_model")]
    pub model: String,

    #[serde(default = "default_claude_tools")]
    pub tools: Vec<String>,

    #[serde(default = "default_permission_mode")]
    pub permission_mode: String,
}

impl Default for ClaudeCliConfig {
    fn default() -> Self {
        Self {
            binary: default_claude_binary(),
            model: default_claude_model(),
            tools: default_claude_tools(),
            permission_mode: default_permission_mode(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct CodexCliConfig {
    #[serde(default = "default_codex_binary")]
    pub binary: PathBuf,

    #[serde(default = "default_codex_model")]
    pub model: String,
}

impl Default for CodexCliConfig {
    fn default() -> Self {
        Self {
            binary: default_codex_binary(),
            model: default_codex_model(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct RetryConfig {
    #[serde(default = "default_max_attempts")]
    pub max_attempts: u32,

    #[serde(default = "default_backoff_base_ms")]
    pub backoff_base_ms: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: default_max_attempts(),
            backoff_base_ms: default_backoff_base_ms(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct PostProcessConfig {
    #[serde(default = "default_false")]
    pub enabled: bool,

    /// Which CLI tool (e.g., claude_cli or codex_cli) to run the reducer/merger step
    #[serde(default = "default_postprocess_tool")]
    pub tool: String,

    /// Path to the prompt file for the reduction step
    #[serde(default = "default_postprocess_prompt")]
    pub prompt_file: PathBuf,

    /// Timeout in seconds for the postprocess CLI invocation
    #[serde(default = "default_postprocess_timeout")]
    pub timeout_sec: u64,

    /// Minimum number of findings required to run postprocessing
    #[serde(default = "default_postprocess_min_findings")]
    pub min_findings: usize,
}

impl Default for PostProcessConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            tool: default_postprocess_tool(),
            prompt_file: default_postprocess_prompt(),
            timeout_sec: default_postprocess_timeout(),
            min_findings: default_postprocess_min_findings(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct Scope {
    pub paths: Vec<PathBuf>,

    #[serde(default)]
    pub include: Vec<String>,

    #[serde(default)]
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct Reviewer {
    pub id: String,

    pub name: String,

    #[serde(default = "default_true")]
    pub enabled: bool,

    pub provider: Provider,

    pub scopes: Vec<String>,

    pub prompt_file: PathBuf,

    #[serde(default)]
    pub priority_default: Priority,

    #[serde(default)]
    pub max_files: Option<usize>,

    #[serde(default)]
    pub timeout_sec: Option<u64>,

    #[serde(default)]
    pub command_override: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    ClaudeCli,
    CodexCli,
}

impl std::fmt::Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Provider::ClaudeCli => write!(f, "claude_cli"),
            Provider::CodexCli => write!(f, "codex_cli"),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    P0,
    #[default]
    P1,
    P2,
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Priority::P0 => write!(f, "p0"),
            Priority::P1 => write!(f, "p1"),
            Priority::P2 => write!(f, "p2"),
        }
    }
}

impl std::str::FromStr for Priority {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "p0" | "critical" | "high" => Ok(Priority::P0),
            "p1" | "medium" => Ok(Priority::P1),
            "p2" | "low" => Ok(Priority::P2),
            _ => Err(format!("Unknown priority: {}", s)),
        }
    }
}
