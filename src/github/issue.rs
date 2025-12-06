use super::dedupe::{DedupeChecker, DedupeResult};
use crate::error::GitHubError;
use crate::parser::Finding;
use std::process::Command;
use tracing::debug;

#[derive(Debug)]
#[allow(dead_code)]
pub enum IssueResult {
    Created { url: String },
    Skipped { issue_number: u64 },
    Commented { issue_number: u64 }, // Future: add comment to existing
    Reopened { issue_number: u64 },  // Future: reopen closed issue
}

pub struct IssueCreator {
    repo: Option<String>,
    labels: Vec<String>,
    assignees: Vec<String>,
    dedupe: bool,
    dedupe_action: crate::config::DedupeAction,
}

impl IssueCreator {
    pub fn new(
        repo: Option<String>,
        dedupe: bool,
        dedupe_action: crate::config::DedupeAction,
        labels: Vec<String>,
        assignees: Vec<String>,
    ) -> Result<Self, GitHubError> {
        Ok(Self {
            repo,
            labels,
            assignees,
            dedupe,
            dedupe_action,
        })
    }

    pub async fn create_or_update(
        &self,
        finding: &Finding,
        reviewer_id: &str,
    ) -> Result<IssueResult, GitHubError> {
        let fingerprint = finding.fingerprint(reviewer_id);

        if self.dedupe {
            let checker = DedupeChecker::new(self.repo.clone());

            match checker.check(&fingerprint)? {
                DedupeResult::Found {
                    issue_number,
                    state,
                } => {
                    debug!(
                        "Found existing issue #{} (state: {:?}) for fingerprint {}",
                        issue_number, state, fingerprint
                    );
                    match self.dedupe_action {
                        crate::config::DedupeAction::Skip => {
                            return Ok(IssueResult::Skipped { issue_number });
                        }
                        crate::config::DedupeAction::Comment => {
                            self.comment_issue(issue_number, finding, reviewer_id, &fingerprint)?;
                            return Ok(IssueResult::Commented { issue_number });
                        }
                        crate::config::DedupeAction::Reopen => {
                            if state == crate::github::dedupe::IssueState::Closed {
                                self.reopen_issue(issue_number)?;
                            }
                            self.comment_issue(issue_number, finding, reviewer_id, &fingerprint)?;
                            return Ok(IssueResult::Reopened { issue_number });
                        }
                    }
                }
                DedupeResult::NotFound => {
                    debug!("No existing issue found for fingerprint {}", fingerprint);
                }
            }
        }

        // Create new issue
        let url = self.create_new_issue(finding, reviewer_id, &fingerprint)?;
        Ok(IssueResult::Created { url })
    }

    fn create_new_issue(
        &self,
        finding: &Finding,
        reviewer_id: &str,
        fingerprint: &str,
    ) -> Result<String, GitHubError> {
        let title = format!("[{}] {}", finding.priority, finding.title);
        let body = self.format_body(finding, reviewer_id, fingerprint);

        let mut cmd = Command::new("gh");
        cmd.arg("issue")
            .arg("create")
            .arg("--title")
            .arg(&title)
            .arg("--body")
            .arg(&body);

        if let Some(repo) = &self.repo {
            cmd.arg("--repo").arg(repo);
        }

        for label in &self.labels {
            cmd.arg("--label").arg(label);
        }

        for assignee in &self.assignees {
            cmd.arg("--assignee").arg(assignee);
        }

        // Priority label
        cmd.arg("--label").arg(finding.priority.to_string());

        let output = cmd.output().map_err(GitHubError::Io)?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Check for label errors and provide helpful message
            if stderr.contains("label") && stderr.contains("not found") {
                return Err(GitHubError::CreateFailed(format!(
                    "{}. Run `polyrev init --repo <owner/repo>` to create required labels",
                    stderr.trim()
                )));
            }
            return Err(GitHubError::CreateFailed(stderr.to_string()));
        }

        let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(url)
    }

    fn comment_issue(
        &self,
        issue_number: u64,
        finding: &Finding,
        reviewer_id: &str,
        fingerprint: &str,
    ) -> Result<(), GitHubError> {
        let body = format!(
            "Update for fingerprint {} ({}:{} by {}):\n\n{}",
            fingerprint,
            finding.file.display(),
            finding.line,
            reviewer_id,
            finding.description
        );

        let mut cmd = Command::new("gh");
        cmd.arg("issue")
            .arg("comment")
            .arg(issue_number.to_string())
            .arg("--body")
            .arg(&body);

        if let Some(repo) = &self.repo {
            cmd.arg("--repo").arg(repo);
        }

        let output = cmd.output().map_err(GitHubError::Io)?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(GitHubError::GhCli(stderr.to_string()));
        }

        Ok(())
    }

    fn reopen_issue(&self, issue_number: u64) -> Result<(), GitHubError> {
        let mut cmd = Command::new("gh");
        cmd.arg("issue").arg("reopen").arg(issue_number.to_string());
        if let Some(repo) = &self.repo {
            cmd.arg("--repo").arg(repo);
        }

        let output = cmd.output().map_err(GitHubError::Io)?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(GitHubError::GhCli(stderr.to_string()));
        }
        Ok(())
    }

    fn format_body(&self, finding: &Finding, reviewer_id: &str, fingerprint: &str) -> String {
        let acceptance_criteria = if finding.acceptance_criteria.is_empty() {
            "- [ ] Address finding\n- [ ] Add test coverage".to_string()
        } else {
            finding
                .acceptance_criteria
                .iter()
                .map(|c| format!("- [ ] {}", c))
                .collect::<Vec<_>>()
                .join("\n")
        };

        let references = if finding.references.is_empty() {
            "N/A".to_string()
        } else {
            finding
                .references
                .iter()
                .map(|r| format!("- {}", r))
                .collect::<Vec<_>>()
                .join("\n")
        };

        let snippet = finding.snippet.as_deref().unwrap_or("N/A");

        format!(
            r#"<!-- polyrev:fp:{fingerprint} -->

## Description

{description}

## Location

| Field | Value |
|-------|-------|
| **File** | `{file}:{line}` |
| **Type** | `{finding_type}` |
| **Reviewer** | `{reviewer_id}` |
| **Priority** | `{priority}` |

## Code

```
{snippet}
```

## Remediation

{remediation}

## Acceptance Criteria

{acceptance_criteria}

## References

{references}

---
*Generated by [polyrev](https://github.com/rk23/polyrev)*
"#,
            fingerprint = fingerprint,
            description = finding.description,
            file = finding.file.display(),
            line = finding.line,
            finding_type = if finding.finding_type.is_empty() {
                "general"
            } else {
                &finding.finding_type
            },
            reviewer_id = reviewer_id,
            priority = finding.priority,
            snippet = snippet,
            remediation = finding.remediation,
            acceptance_criteria = acceptance_criteria,
            references = references,
        )
    }
}
