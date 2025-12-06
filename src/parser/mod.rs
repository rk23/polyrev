mod finding;
mod json;
mod markdown;

pub use finding::Finding;

use crate::config::Priority;

/// Parse findings from provider output
/// Tries JSON first, then falls back to markdown table parsing
pub fn parse_findings(raw: &str, reviewer_id: &str, default_priority: Priority) -> Vec<Finding> {
    // Try JSON first
    if let Some(findings) = json::try_parse_json(raw) {
        return findings;
    }

    // Fallback: markdown table
    if let Some(findings) = markdown::try_parse_markdown_table(raw, reviewer_id, default_priority) {
        return findings;
    }

    tracing::warn!(
        "Could not parse findings from output for reviewer {}",
        reviewer_id
    );
    Vec::new()
}
