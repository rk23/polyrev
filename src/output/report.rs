use crate::error::OutputError;
use crate::runner::{ReviewerResult, ReviewerStatus};
use std::fs;
use std::path::Path;

/// Write a single reviewer's report immediately
pub fn write_reviewer_report(
    report_dir: &Path,
    result: &ReviewerResult,
) -> Result<(), OutputError> {
    // Ensure directory exists
    fs::create_dir_all(report_dir).map_err(OutputError::CreateDir)?;
    let mut content = String::new();

    // Header
    content.push_str(&format!("# {}\n\n", result.reviewer_name));

    // Metadata table
    content.push_str("| Metric | Value |\n");
    content.push_str("|--------|-------|\n");
    content.push_str(&format!("| Status | {} |\n", format_status(&result.status)));
    content.push_str(&format!(
        "| Duration | {:.1}s |\n",
        result.duration.as_secs_f64()
    ));
    content.push_str(&format!("| Files Scanned | {} |\n", result.files_scanned));

    // Finding counts
    let p0_count = result
        .findings
        .iter()
        .filter(|f| f.priority == crate::config::Priority::P0)
        .count();
    let p1_count = result
        .findings
        .iter()
        .filter(|f| f.priority == crate::config::Priority::P1)
        .count();
    let p2_count = result
        .findings
        .iter()
        .filter(|f| f.priority == crate::config::Priority::P2)
        .count();

    content.push_str(&format!("| p0 (Critical) | {} |\n", p0_count));
    content.push_str(&format!("| p1 (High) | {} |\n", p1_count));
    content.push_str(&format!("| p2 (Medium) | {} |\n", p2_count));
    content.push_str("\n---\n\n");

    // Findings
    if result.findings.is_empty() {
        content.push_str("*No findings*\n");
    } else {
        content.push_str("## Findings\n\n");

        for finding in &result.findings {
            content.push_str(&format!("### [{}] {}\n\n", finding.priority, finding.title));
            if finding.line > 0 {
                content.push_str(&format!(
                    "- **File:** `{}:{}`\n",
                    finding.file.display(),
                    finding.line
                ));
            } else {
                content.push_str(&format!("- **File:** `{}`\n", finding.file.display()));
            }
            if !finding.finding_type.is_empty() {
                content.push_str(&format!("- **Type:** `{}`\n", finding.finding_type));
            }
            content.push('\n');
            content.push_str(&format!("{}\n\n", finding.description));

            if let Some(snippet) = &finding.snippet {
                content.push_str("**Code:**\n");
                content.push_str(&format!("```\n{}\n```\n\n", snippet));
            }

            content.push_str(&format!("**Remediation:** {}\n\n", finding.remediation));

            if !finding.acceptance_criteria.is_empty() {
                content.push_str("**Acceptance Criteria:**\n");
                for criterion in &finding.acceptance_criteria {
                    content.push_str(&format!("- [ ] {}\n", criterion));
                }
                content.push('\n');
            }

            if !finding.references.is_empty() {
                content.push_str("**References:**\n");
                for reference in &finding.references {
                    content.push_str(&format!("- {}\n", reference));
                }
                content.push('\n');
            }

            content.push_str("---\n\n");
        }
    }

    // Write markdown report
    let report_path = report_dir.join(format!("{}.md", result.reviewer_id));
    fs::write(&report_path, &content).map_err(OutputError::WriteReport)?;

    // Also write findings as JSON for issue creation
    if !result.findings.is_empty() {
        let findings_path = report_dir.join(format!("{}.findings.json", result.reviewer_id));
        let json = serde_json::to_string_pretty(&result.findings)?;
        fs::write(&findings_path, json).map_err(OutputError::WriteReport)?;
    }

    Ok(())
}

fn format_status(status: &ReviewerStatus) -> String {
    match status {
        ReviewerStatus::Completed => "✅ Completed".to_string(),
        ReviewerStatus::Skipped { reason } => format!("⏭️ Skipped ({})", reason),
        ReviewerStatus::TimedOut => "⏱️ Timed Out".to_string(),
        ReviewerStatus::Failed { error } => format!("❌ Failed ({})", error),
    }
}
