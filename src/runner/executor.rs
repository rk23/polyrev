use crate::config::{Config, Reviewer};
use crate::discovery::{chunk_files, discover_files_for_reviewer};
use crate::error::RunnerError;
use crate::parser::{parse_findings, Finding};
use crate::provider::{create_runner, SessionInfo};
use std::path::PathBuf;
use std::time::Duration;
use tracing::{debug, info, warn};
use uuid::Uuid;

use super::retry::retry_with_backoff;
use super::{ReviewerResult, ReviewerStatus};

/// Build a chunk-aware prompt that instructs Claude to accumulate or output
fn build_chunk_prompt(
    base_prompt: &str,
    chunk_idx: usize,
    total_chunks: usize,
    files: &[PathBuf],
) -> String {
    let file_list = files
        .iter()
        .map(|f| format!("- {}", f.display()))
        .collect::<Vec<_>>()
        .join("\n");

    if total_chunks == 1 {
        // Single chunk - no special instructions needed
        base_prompt.to_string()
    } else if chunk_idx + 1 == total_chunks {
        // Final chunk - request output
        format!(
            "{}\n\n---\n\n\
            **[CHUNKED REVIEW: {}/{} - FINAL CHUNK]**\n\n\
            You have now received ALL files across {} chunks. \
            Analyze ALL files from ALL chunks together and output your findings as JSON.\n\n\
            Files in this final chunk:\n{}",
            base_prompt,
            chunk_idx + 1,
            total_chunks,
            total_chunks,
            file_list
        )
    } else {
        // Accumulating chunk - suppress output
        format!(
            "{}\n\n---\n\n\
            **[CHUNKED REVIEW: {}/{} - ACCUMULATING]**\n\n\
            This review is split into {} chunks. Read and index these files. \
            Do NOT output findings yet - wait for the final chunk.\n\n\
            Reply ONLY with: `Chunk {}/{} received. {} files indexed.`\n\n\
            Files in this chunk:\n{}",
            base_prompt,
            chunk_idx + 1,
            total_chunks,
            total_chunks,
            chunk_idx + 1,
            total_chunks,
            files.len(),
            file_list
        )
    }
}

pub async fn execute_reviewer(
    config: &Config,
    reviewer: &Reviewer,
    diff_base: Option<&str>,
) -> Result<ReviewerResult, RunnerError> {
    let start = std::time::Instant::now();

    // Discover files for this reviewer
    let files = match discover_files_for_reviewer(config, reviewer, diff_base) {
        Ok(f) => f,
        Err(e) => {
            return Ok(ReviewerResult {
                reviewer_id: reviewer.id.clone(),
                reviewer_name: reviewer.name.clone(),
                status: ReviewerStatus::Failed {
                    error: e.to_string(),
                },
                files_scanned: 0,
                findings: Vec::new(),
                duration: start.elapsed(),
            });
        }
    };

    if files.is_empty() {
        info!("Skipping {} - no matching files", reviewer.id);
        return Ok(ReviewerResult {
            reviewer_id: reviewer.id.clone(),
            reviewer_name: reviewer.name.clone(),
            status: ReviewerStatus::Skipped {
                reason: "no matching files".to_string(),
            },
            files_scanned: 0,
            findings: Vec::new(),
            duration: start.elapsed(),
        });
    }

    info!("Reviewer {} found {} files", reviewer.id, files.len());

    // Load prompt template (relative to target if not absolute)
    let prompt_path = if reviewer.prompt_file.is_absolute() {
        reviewer.prompt_file.clone()
    } else {
        config.target.join(&reviewer.prompt_file)
    };

    let prompt = match std::fs::read_to_string(&prompt_path) {
        Ok(p) => p,
        Err(e) => {
            return Ok(ReviewerResult {
                reviewer_id: reviewer.id.clone(),
                reviewer_name: reviewer.name.clone(),
                status: ReviewerStatus::Failed {
                    error: format!(
                        "Failed to read prompt file ({}): {}",
                        prompt_path.display(),
                        e
                    ),
                },
                files_scanned: 0,
                findings: Vec::new(),
                duration: start.elapsed(),
            });
        }
    };

    // Create runner for this reviewer's provider
    let runner = create_runner(config, reviewer);

    // Get timeout and max_files settings
    let timeout = Duration::from_secs(reviewer.timeout_sec.unwrap_or(config.timeout_sec));
    let max_files = reviewer.max_files.unwrap_or(config.max_files);

    // Chunk files if needed
    let chunks = chunk_files(&files, max_files);
    let total_chunks = chunks.len();
    debug!(
        "Reviewer {} split into {} chunks",
        reviewer.id, total_chunks
    );

    let mut all_findings: Vec<Finding> = Vec::new();
    let mut chunk_successes = 0usize;
    let mut chunk_failures = 0usize;
    let mut last_error: Option<String> = None;
    let files_scanned = files.len();

    // Session ID for multi-chunk runs
    // Claude: generate one to enable --session-id/--resume
    // Codex: will be filled from provider output after first chunk
    let mut session_id: Option<String> =
        if reviewer.provider == crate::config::Provider::ClaudeCli && total_chunks > 1 {
            Some(Uuid::new_v4().to_string())
        } else {
            None
        };

    // Execute each chunk with retries
    for (chunk_idx, chunk) in chunks.iter().enumerate() {
        debug!(
            "Reviewer {} executing chunk {}/{} ({} files)",
            reviewer.id,
            chunk_idx + 1,
            total_chunks,
            chunk.len()
        );

        // Build chunk-aware prompt
        let chunk_prompt = build_chunk_prompt(&prompt, chunk_idx, total_chunks, chunk);

        // Build session info for this chunk
        let session_info = session_id.as_ref().map(|sid| SessionInfo {
            session_id: Some(sid.clone()),
            is_resume: chunk_idx > 0,
        });

        let chunk_clone = chunk.clone();
        let runner_clone = runner.clone();

        let result = retry_with_backoff(&config.retry, || {
            let prompt_for_retry = chunk_prompt.clone();
            let files_for_retry = chunk_clone.clone();
            let runner_for_retry = runner_clone.clone();
            let session_for_retry = session_info.clone();
            async move {
                runner_for_retry
                    .execute(
                        &prompt_for_retry,
                        &files_for_retry,
                        timeout,
                        session_for_retry.as_ref(),
                    )
                    .await
            }
        })
        .await;

        match result {
            Ok(output) => {
                debug!(
                    "Reviewer {} chunk {} completed in {:?}",
                    reviewer.id,
                    chunk_idx + 1,
                    output.duration
                );

                // Capture provider session id for subsequent chunks
                if session_id.is_none() {
                    if let Some(sid) = output.session_id.clone() {
                        session_id = Some(sid);
                        debug!("Reviewer {} obtained session id", reviewer.id);
                    }
                }

                // Only parse findings from the final chunk (or single chunk)
                if chunk_idx + 1 == total_chunks {
                    let findings =
                        parse_findings(&output.stdout, &reviewer.id, reviewer.priority_default);
                    all_findings.extend(findings);
                } else {
                    debug!(
                        "Reviewer {} chunk {} acknowledged: {}",
                        reviewer.id,
                        chunk_idx + 1,
                        output.stdout.lines().next().unwrap_or("(no response)")
                    );
                }
                chunk_successes += 1;
            }
            Err(e) => {
                warn!(
                    "Reviewer {} chunk {} failed after retries: {}",
                    reviewer.id,
                    chunk_idx + 1,
                    e
                );
                chunk_failures += 1;
                last_error = Some(e.to_string());
                // For multi-chunk with session, failing early chunk breaks the chain
                if total_chunks > 1 && chunk_idx < total_chunks - 1 {
                    warn!(
                        "Reviewer {} aborting remaining chunks due to session failure",
                        reviewer.id
                    );
                    break;
                }
            }
        }
    }

    let status = if chunk_successes == 0 {
        ReviewerStatus::Failed {
            error: last_error.unwrap_or_else(|| "all chunks failed".to_string()),
        }
    } else if chunk_failures > 0 {
        ReviewerStatus::Failed {
            error: format!(
                "{} of {} chunks failed; partial results returned",
                chunk_failures,
                chunk_failures + chunk_successes
            ),
        }
    } else {
        ReviewerStatus::Completed
    };

    Ok(ReviewerResult {
        reviewer_id: reviewer.id.clone(),
        reviewer_name: reviewer.name.clone(),
        status,
        files_scanned,
        findings: all_findings,
        duration: start.elapsed(),
    })
}
