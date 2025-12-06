use super::{ProviderOutput, Runner, SessionInfo};
use crate::error::ProviderError;
use async_trait::async_trait;
use serde_json::Value;
use std::path::PathBuf;
use std::time::Duration;
use tempfile::NamedTempFile;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::time::timeout as tokio_timeout;

pub struct CodexRunner {
    pub binary: PathBuf,
    pub model: String,
    pub working_dir: PathBuf,
}

#[async_trait]
impl Runner for CodexRunner {
    fn name(&self) -> &'static str {
        "codex_cli"
    }

    async fn execute(
        &self,
        prompt: &str,
        files: &[PathBuf],
        timeout: Duration,
        session: Option<&SessionInfo>,
    ) -> Result<ProviderOutput, ProviderError> {
        let file_list = files
            .iter()
            .map(|f| f.display().to_string())
            .collect::<Vec<_>>()
            .join("\n");

        let full_prompt = format!("{}\n\n## Files to Review\n```\n{}\n```", prompt, file_list);

        // Capture final assistant message to a temp file (only for initial exec)
        let mut _out_file: Option<NamedTempFile> = None;
        let mut out_path: Option<PathBuf> = None;

        // Use string for PATH lookup if not an absolute/relative path
        let binary_str = self.binary.to_string_lossy();
        let mut cmd = if binary_str.contains('/') || binary_str.contains('\\') {
            Command::new(&self.binary)
        } else {
            Command::new(binary_str.as_ref())
        };
        let is_resume = session.map(|s| s.is_resume).unwrap_or(false);

        // Choose base command (resume if session provided). Codex resume does not accept --model/--json flags.
        if is_resume {
            cmd.arg("exec").arg("resume");
            if let Some(sess) = session {
                if let Some(id) = &sess.session_id {
                    cmd.arg(id);
                }
            }
        } else {
            cmd.arg("exec");
            cmd.arg("--model").arg(&self.model);
            // Ask for JSON events and write last message to a file for easy parsing
            cmd.arg("--json");
            let tmp = NamedTempFile::new().map_err(ProviderError::Io)?;
            out_path = Some(tmp.path().to_path_buf());
            cmd.arg("--output-last-message")
                .arg(out_path.as_ref().unwrap());
            _out_file = Some(tmp);
        }

        // Read prompt from stdin
        cmd.arg("-");

        cmd.current_dir(&self.working_dir);

        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let start = std::time::Instant::now();

        let mut child = cmd.spawn().map_err(ProviderError::Io)?;

        // Write prompt to stdin
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(full_prompt.as_bytes())
                .await
                .map_err(ProviderError::Io)?;
            stdin.shutdown().await.map_err(ProviderError::Io)?;
        }

        let output = tokio_timeout(timeout, child.wait_with_output())
            .await
            .map_err(|_| ProviderError::Timeout(timeout))?
            .map_err(ProviderError::Io)?;

        // Extract thread/session id from JSONL stdout (only on initial run)
        let mut session_id: Option<String> = None;
        let final_stdout = if is_resume {
            String::from_utf8_lossy(&output.stdout).to_string()
        } else {
            for line in String::from_utf8_lossy(&output.stdout).lines() {
                if let Ok(val) = serde_json::from_str::<Value>(line) {
                    if let Some(tid) = val.get("thread_id").and_then(|v| v.as_str()) {
                        session_id = Some(tid.to_string());
                    }
                }
            }
            if let Some(path) = out_path.as_ref() {
                std::fs::read_to_string(path).unwrap_or_else(|_| "".to_string())
            } else {
                String::new()
            }
        };

        let result = ProviderOutput {
            stdout: final_stdout,
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            duration: start.elapsed(),
            exit_code: output.status.code().unwrap_or(-1),
            session_id,
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
