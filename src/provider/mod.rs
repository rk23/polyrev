mod claude;
mod codex;

pub use claude::ClaudeRunner;
pub use codex::CodexRunner;

use crate::config::{Config, Provider, Reviewer};
use crate::error::ProviderError;
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug)]
pub struct ProviderOutput {
    pub stdout: String,
    pub stderr: String,
    pub duration: Duration,
    pub exit_code: i32,
    pub session_id: Option<String>,
}

/// Session management for multi-chunk execution
#[derive(Debug, Clone, Default)]
pub struct SessionInfo {
    /// Session ID for resumable conversations (UUID)
    pub session_id: Option<String>,
    /// Whether to resume an existing session (true) or start new (false)
    pub is_resume: bool,
}

#[async_trait]
pub trait Runner: Send + Sync {
    #[allow(dead_code)]
    fn name(&self) -> &'static str;

    async fn execute(
        &self,
        prompt: &str,
        files: &[PathBuf],
        timeout: Duration,
        session: Option<&SessionInfo>,
    ) -> Result<ProviderOutput, ProviderError>;
}

/// Create a runner based on the reviewer's provider configuration
pub fn create_runner(config: &Config, reviewer: &Reviewer) -> Arc<dyn Runner> {
    match reviewer.provider {
        Provider::ClaudeCli => Arc::new(ClaudeRunner {
            binary: config.providers.claude_cli.binary.clone(),
            tools: config.providers.claude_cli.tools.clone(),
            permission_mode: config.providers.claude_cli.permission_mode.clone(),
            working_dir: config.target.clone(),
        }),
        Provider::CodexCli => Arc::new(CodexRunner {
            binary: config.providers.codex_cli.binary.clone(),
            model: config.providers.codex_cli.model.clone(),
            working_dir: config.target.clone(),
        }),
    }
}
