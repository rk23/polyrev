use super::{ProviderOutput, Runner, SessionInfo};
use crate::error::ProviderError;
use async_trait::async_trait;
use std::path::PathBuf;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout as tokio_timeout;

pub struct ClaudeRunner {
    pub binary: PathBuf,
    pub model: String,
    pub tools: Vec<String>,
    pub permission_mode: String,
    pub working_dir: PathBuf,
}

#[async_trait]
impl Runner for ClaudeRunner {
    fn name(&self) -> &'static str {
        "claude_cli"
    }

    async fn execute(
        &self,
        prompt: &str,
        files: &[PathBuf],
        timeout: Duration,
        session: Option<&SessionInfo>,
    ) -> Result<ProviderOutput, ProviderError> {
        // Build file list into prompt context
        let file_list = files
            .iter()
            .map(|f| f.display().to_string())
            .collect::<Vec<_>>()
            .join("\n");

        let full_prompt = format!("{}\n\n## Files to Review\n```\n{}\n```", prompt, file_list);

        // Build command - use string for PATH lookup if not an absolute/relative path
        let binary_str = self.binary.to_string_lossy();
        let mut cmd = if binary_str.contains('/') || binary_str.contains('\\') {
            Command::new(&self.binary)
        } else {
            // Plain command name - let shell find it in PATH
            Command::new(binary_str.as_ref())
        };

        cmd.current_dir(&self.working_dir);

        // Ensure subscription auth is used (not API key)
        cmd.env_remove("ANTHROPIC_API_KEY");

        // Handle session management for multi-chunk execution
        if let Some(sess) = session {
            if let Some(ref session_id) = sess.session_id {
                if sess.is_resume {
                    // Resume existing session
                    cmd.arg("--resume").arg(session_id);
                } else {
                    // Start new session with specific ID
                    cmd.arg("--session-id").arg(session_id);
                }
            }
        }

        cmd.arg("-p")
            .arg(&full_prompt)
            .arg("--model")
            .arg(&self.model)
            .arg("--output-format")
            .arg("json")
            .arg("--allowedTools")
            .arg(self.tools.join(","))
            .arg("--permission-mode")
            .arg(&self.permission_mode);

        let start = std::time::Instant::now();

        let output = tokio_timeout(timeout, cmd.output())
            .await
            .map_err(|_| ProviderError::Timeout(timeout))?
            .map_err(ProviderError::Io)?;

        let result = ProviderOutput {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            duration: start.elapsed(),
            exit_code: output.status.code().unwrap_or(-1),
            session_id: None,
        };

        if !output.status.success() {
            return Err(ProviderError::NonZeroExit {
                code: result.exit_code,
                stderr: result.stderr.clone(),
            });
        }

        Ok(result)
    }
}
