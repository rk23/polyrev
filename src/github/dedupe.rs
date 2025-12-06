use crate::error::GitHubError;
use serde::Deserialize;
use std::process::Command;

pub struct DedupeChecker {
    pub repo: Option<String>,
}

#[derive(Debug)]
pub enum DedupeResult {
    NotFound,
    Found {
        issue_number: u64,
        state: IssueState,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum IssueState {
    Open,
    Closed,
}

#[derive(Deserialize)]
struct IssueInfo {
    number: u64,
    state: String,
}

impl DedupeChecker {
    pub fn new(repo: Option<String>) -> Self {
        Self { repo }
    }

    /// Check if an issue with this fingerprint already exists
    pub fn check(&self, fingerprint: &str) -> Result<DedupeResult, GitHubError> {
        let search = format!("polyrev:fp:{} in:body", fingerprint);

        let mut cmd = Command::new("gh");
        cmd.arg("issue")
            .arg("list")
            .arg("--state")
            .arg("all") // include closed
            .arg("--search")
            .arg(&search)
            .arg("--json")
            .arg("number,state")
            .arg("--limit")
            .arg("1");

        if let Some(repo) = &self.repo {
            cmd.arg("--repo").arg(repo);
        }

        let output = cmd.output().map_err(GitHubError::Io)?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(GitHubError::GhCli(stderr.to_string()));
        }

        let json: Vec<IssueInfo> = serde_json::from_slice(&output.stdout)
            .map_err(|e| GitHubError::ParseOutput(e.to_string()))?;

        match json.first() {
            None => Ok(DedupeResult::NotFound),
            Some(info) => Ok(DedupeResult::Found {
                issue_number: info.number,
                state: if info.state == "OPEN" {
                    IssueState::Open
                } else {
                    IssueState::Closed
                },
            }),
        }
    }
}
