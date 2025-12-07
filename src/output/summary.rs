use crate::config::Priority;
use crate::error::OutputError;
use crate::runner::{ReviewerStatus, RunReport};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
pub struct SummaryReport {
    pub timestamp: String,
    pub target: String,
    pub duration_sec: f64,
    pub reviewers: Vec<ReviewerSummary>,
    pub totals: HashMap<String, usize>,
    pub skipped: Vec<String>,
    pub failed: Vec<String>,
    pub exit_code: i32,
    pub report_dir: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReviewerSummary {
    pub id: String,
    pub name: String,
    pub status: String,
    pub duration_sec: f64,
    pub files_scanned: usize,
    pub findings: HashMap<String, usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

pub fn write_summary(
    report_dir: &Path,
    run_report: &RunReport,
    target: &Path,
) -> Result<(), OutputError> {
    // Ensure report directory exists (needed when all reviewers were skipped)
    fs::create_dir_all(report_dir).map_err(OutputError::CreateDir)?;

    let summary = build_summary(run_report, report_dir.to_path_buf(), target);

    // Write JSON
    let json_path = report_dir.join("summary.json");
    let json = serde_json::to_string_pretty(&summary)?;
    fs::write(&json_path, json).map_err(OutputError::WriteReport)?;

    // Write Markdown
    let md_path = report_dir.join("summary.md");
    let md = build_summary_markdown(&summary);
    fs::write(&md_path, md).map_err(OutputError::WriteReport)?;

    Ok(())
}

fn build_summary(run_report: &RunReport, report_dir: PathBuf, target: &Path) -> SummaryReport {
    let mut reviewers = Vec::new();
    let mut skipped = Vec::new();
    let mut failed = Vec::new();
    let mut total_p0 = 0;
    let mut total_p1 = 0;
    let mut total_p2 = 0;

    for result in &run_report.reviewer_results {
        let p0 = result
            .findings
            .iter()
            .filter(|f| f.priority == Priority::P0)
            .count();
        let p1 = result
            .findings
            .iter()
            .filter(|f| f.priority == Priority::P1)
            .count();
        let p2 = result
            .findings
            .iter()
            .filter(|f| f.priority == Priority::P2)
            .count();

        total_p0 += p0;
        total_p1 += p1;
        total_p2 += p2;

        let (status_str, reason) = match &result.status {
            ReviewerStatus::Completed => ("completed".to_string(), None),
            ReviewerStatus::Skipped { reason } => {
                skipped.push(result.reviewer_id.clone());
                ("skipped".to_string(), Some(reason.clone()))
            }
            ReviewerStatus::TimedOut => {
                failed.push(result.reviewer_id.clone());
                ("timed_out".to_string(), None)
            }
            ReviewerStatus::Failed { error } => {
                failed.push(result.reviewer_id.clone());
                ("failed".to_string(), Some(error.clone()))
            }
        };

        let mut findings = HashMap::new();
        findings.insert("p0".to_string(), p0);
        findings.insert("p1".to_string(), p1);
        findings.insert("p2".to_string(), p2);

        reviewers.push(ReviewerSummary {
            id: result.reviewer_id.clone(),
            name: result.reviewer_name.clone(),
            status: status_str,
            duration_sec: result.duration.as_secs_f64(),
            files_scanned: result.files_scanned,
            findings,
            reason,
        });
    }

    let mut totals = HashMap::new();
    totals.insert("p0".to_string(), total_p0);
    totals.insert("p1".to_string(), total_p1);
    totals.insert("p2".to_string(), total_p2);

    let exit_code = if total_p0 > 0 { 1 } else { 0 };

    SummaryReport {
        timestamp: Utc::now().to_rfc3339(),
        target: target.display().to_string(),
        duration_sec: run_report.total_duration.as_secs_f64(),
        reviewers,
        totals,
        skipped,
        failed,
        exit_code,
        report_dir,
    }
}

fn build_summary_markdown(summary: &SummaryReport) -> String {
    let mut md = String::new();

    md.push_str("# polyrev Summary\n\n");
    md.push_str(&format!("**Generated:** {}\n", summary.timestamp));
    md.push_str(&format!("**Target:** {}\n", summary.target));
    md.push_str(&format!(
        "**Report Dir:** {}\n",
        summary.report_dir.display()
    ));
    md.push_str(&format!("**Duration:** {:.1}s\n\n", summary.duration_sec));

    // Totals
    md.push_str("## Totals\n\n");
    md.push_str("| Priority | Count |\n");
    md.push_str("|----------|-------|\n");
    md.push_str(&format!(
        "| p0 (Critical) | {} |\n",
        summary.totals.get("p0").unwrap_or(&0)
    ));
    md.push_str(&format!(
        "| p1 (High) | {} |\n",
        summary.totals.get("p1").unwrap_or(&0)
    ));
    md.push_str(&format!(
        "| p2 (Medium) | {} |\n\n",
        summary.totals.get("p2").unwrap_or(&0)
    ));

    // Reviewers table
    md.push_str("## Reviewers\n\n");
    md.push_str("| Reviewer | Status | Findings |\n");
    md.push_str("|----------|--------|----------|\n");

    for reviewer in &summary.reviewers {
        let status_icon = match reviewer.status.as_str() {
            "completed" => "✅",
            "skipped" => "⏭️",
            "timed_out" => "⏱️",
            "failed" => "❌",
            _ => "❓",
        };

        let findings_str = format!(
            "{} p0, {} p1, {} p2",
            reviewer.findings.get("p0").unwrap_or(&0),
            reviewer.findings.get("p1").unwrap_or(&0),
            reviewer.findings.get("p2").unwrap_or(&0),
        );

        let status_str = if let Some(reason) = &reviewer.reason {
            format!("{} {} ({})", status_icon, reviewer.status, reason)
        } else {
            format!("{} {}", status_icon, reviewer.status)
        };

        md.push_str(&format!(
            "| {} | {} | {} |\n",
            reviewer.name, status_str, findings_str
        ));
    }

    // Critical findings summary
    let p0_count = *summary.totals.get("p0").unwrap_or(&0);
    if p0_count > 0 {
        md.push_str("\n## Critical Findings (p0)\n\n");
        md.push_str("See individual reviewer reports for details.\n");
    }

    md
}
