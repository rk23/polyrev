use crate::config::Config;
use crate::error::RunnerError;
use crate::output::write_reviewer_report;
use crate::parser::Finding;
use crate::state::State;
use futures::stream::{FuturesUnordered, StreamExt};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::time::sleep;
use tracing::{info, warn};

use super::executor::execute_reviewer;

#[derive(Debug, Clone)]
pub struct RunOptions {
    pub reviewer_filter: Option<Vec<String>>,
    pub scope_filter: Option<Vec<String>>,
    pub diff_base: Option<String>,
    #[allow(dead_code)]
    pub dry_run: bool,
    pub force: bool,
}

#[derive(Debug)]
pub struct RunReport {
    pub reviewer_results: Vec<ReviewerResult>,
    pub total_duration: Duration,
}

impl RunReport {
    pub fn totals(&self) -> FindingCounts {
        let mut counts = FindingCounts::default();
        for result in &self.reviewer_results {
            counts.p0 += result
                .findings
                .iter()
                .filter(|f| f.priority == crate::config::Priority::P0)
                .count();
            counts.p1 += result
                .findings
                .iter()
                .filter(|f| f.priority == crate::config::Priority::P1)
                .count();
            counts.p2 += result
                .findings
                .iter()
                .filter(|f| f.priority == crate::config::Priority::P2)
                .count();
        }
        counts
    }
}

#[derive(Debug, Default)]
pub struct FindingCounts {
    pub p0: usize,
    pub p1: usize,
    pub p2: usize,
}

#[derive(Debug)]
pub struct ReviewerResult {
    pub reviewer_id: String,
    pub reviewer_name: String,
    pub status: ReviewerStatus,
    pub files_scanned: usize,
    pub findings: Vec<Finding>,
    pub duration: Duration,
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum ReviewerStatus {
    Completed,
    Skipped { reason: String },
    TimedOut, // Future: when timeout is hit
    Failed { error: String },
}

impl std::fmt::Display for ReviewerStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReviewerStatus::Completed => write!(f, "completed"),
            ReviewerStatus::Skipped { reason } => write!(f, "skipped: {}", reason),
            ReviewerStatus::TimedOut => write!(f, "timed_out"),
            ReviewerStatus::Failed { error } => write!(f, "failed: {}", error),
        }
    }
}

pub struct Orchestrator {
    config: Config,
    semaphore: Arc<Semaphore>,
}

impl Orchestrator {
    pub fn new(config: Config) -> Result<Self, RunnerError> {
        let semaphore = Arc::new(Semaphore::new(config.concurrency));
        Ok(Self { config, semaphore })
    }

    pub async fn run(
        &self,
        options: &RunOptions,
        state: &State,
        report_dir: &Path,
    ) -> Result<RunReport, RunnerError> {
        let start = std::time::Instant::now();

        // Filter reviewers based on options
        let all_reviewers: Vec<_> = self
            .config
            .reviewers
            .iter()
            .filter(|r| r.enabled)
            .filter(|r| {
                options
                    .reviewer_filter
                    .as_ref()
                    .map(|f| f.contains(&r.id))
                    .unwrap_or(true)
            })
            .filter(|r| {
                options
                    .scope_filter
                    .as_ref()
                    .map(|f| r.scopes.iter().any(|s| f.contains(s)))
                    .unwrap_or(true)
            })
            .cloned()
            .collect();

        // Separate reviewers into those to run and those to skip (already ran today)
        let mut reviewers = Vec::new();
        let mut skipped_results = Vec::new();

        for reviewer in all_reviewers {
            if !options.force && state.ran_today(&reviewer.id) {
                info!(
                    "Skipping {} - already ran within last 24 hours",
                    reviewer.id
                );
                skipped_results.push(ReviewerResult {
                    reviewer_id: reviewer.id.clone(),
                    reviewer_name: reviewer.name.clone(),
                    status: ReviewerStatus::Skipped {
                        reason: "already ran today".to_string(),
                    },
                    files_scanned: 0,
                    findings: Vec::new(),
                    duration: Duration::ZERO,
                });
            } else {
                reviewers.push(reviewer);
            }
        }

        if reviewers.is_empty() && skipped_results.is_empty() {
            return Err(RunnerError::NoReviewersMatched);
        }

        // If all reviewers were skipped, return early with skipped results
        if reviewers.is_empty() {
            info!("All matching reviewers already ran today. Use --force to re-run.");
            // Return skipped-only report so downstream steps (postprocess/summary) can still run.
            return Ok(RunReport {
                reviewer_results: skipped_results,
                total_duration: start.elapsed(),
            });
        }

        info!(
            "Running {} reviewers with concurrency {}",
            reviewers.len(),
            self.config.concurrency
        );

        let mut futures = FuturesUnordered::new();
        let launch_delay = Duration::from_millis(self.config.launch_delay_ms);

        for (idx, reviewer) in reviewers.into_iter().enumerate() {
            // Small delay between launches to avoid burst rate limits
            if idx > 0 && launch_delay > Duration::ZERO {
                sleep(launch_delay).await;
            }

            let permit = self.semaphore.clone().acquire_owned().await?;
            let config = self.config.clone();
            let diff_base = options.diff_base.clone();

            futures.push(tokio::spawn(async move {
                let _permit = permit; // hold until done
                execute_reviewer(&config, &reviewer, diff_base.as_deref()).await
            }));
        }

        let mut results = skipped_results; // Start with skipped reviewers
        while let Some(result) = futures.next().await {
            match result {
                Ok(Ok(report)) => {
                    info!(
                        "Completed {}: {} findings ({:?})",
                        report.reviewer_id,
                        report.findings.len(),
                        report.status
                    );

                    // Write report immediately (streaming mode)
                    if let Err(e) = write_reviewer_report(report_dir, &report) {
                        warn!("Failed to write report for {}: {}", report.reviewer_id, e);
                    } else {
                        info!(
                            "Wrote report: {}/{}.md",
                            report_dir.display(),
                            report.reviewer_id
                        );
                    }

                    results.push(report);
                }
                Ok(Err(e)) => {
                    warn!("Reviewer execution failed: {}", e);
                }
                Err(e) => {
                    warn!("Task panicked: {}", e);
                }
            }
        }

        Ok(RunReport {
            reviewer_results: results,
            total_duration: start.elapsed(),
        })
    }
}
