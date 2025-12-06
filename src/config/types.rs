use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use super::defaults::*;

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
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum DedupeAction {
    #[default]
    Skip,
    Comment,
    Reopen,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct ProvidersConfig {
    #[serde(default)]
    pub claude_cli: ClaudeCliConfig,

    #[serde(default)]
    pub codex_cli: CodexCliConfig,
}

impl Default for ProvidersConfig {
    fn default() -> Self {
        Self {
            claude_cli: ClaudeCliConfig::default(),
            codex_cli: CodexCliConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct ClaudeCliConfig {
    #[serde(default = "default_claude_binary")]
    pub binary: PathBuf,

    #[serde(default = "default_claude_tools")]
    pub tools: Vec<String>,

    #[serde(default = "default_permission_mode")]
    pub permission_mode: String,
}

impl Default for ClaudeCliConfig {
    fn default() -> Self {
        Self {
            binary: default_claude_binary(),
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
