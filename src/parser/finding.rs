use crate::config::Priority;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Finding {
    pub id: String,

    #[serde(default, alias = "type")]
    pub finding_type: String,

    pub title: String,

    #[serde(default)]
    pub priority: Priority,

    pub file: PathBuf,

    #[serde(default)]
    pub line: u32, // 0 means no specific line

    #[serde(default)]
    pub snippet: Option<String>,

    pub description: String,

    #[serde(default, alias = "recommendation")]
    pub remediation: String,

    #[serde(default)]
    pub acceptance_criteria: Vec<String>,

    #[serde(default)]
    pub references: Vec<String>,
}

impl Finding {
    /// Normalize snippet for stable fingerprinting
    /// Removes variable whitespace that might differ between runs
    fn normalize_snippet(&self) -> String {
        self.snippet
            .as_deref()
            .unwrap_or("")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Generate deterministic fingerprint for deduplication
    /// Uses: reviewer_id | relative_file | line | finding_type | normalized_snippet
    pub fn fingerprint(&self, reviewer_id: &str) -> String {
        let normalized_snippet = self.normalize_snippet();
        let input = format!(
            "{}|{}|{}|{}|{}",
            reviewer_id,
            self.file.display(),
            self.line,
            self.finding_type,
            normalized_snippet,
        );
        let hash = Sha256::digest(input.as_bytes());
        format!("{:x}", hash)[..12].to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fingerprint_stability() {
        let finding = Finding {
            id: "TEST-001".to_string(),
            finding_type: "sql-injection".to_string(),
            title: "SQL Injection".to_string(),
            priority: Priority::P0,
            file: PathBuf::from("src/db.py"),
            line: 42,
            snippet: Some("query = f\"SELECT * FROM users WHERE id = {id}\"".to_string()),
            description: "Bad".to_string(),
            remediation: "Fix it".to_string(),
            acceptance_criteria: vec![],
            references: vec![],
        };

        let fp1 = finding.fingerprint("security-python");
        let fp2 = finding.fingerprint("security-python");

        assert_eq!(fp1, fp2);
        assert_eq!(fp1.len(), 12);
    }

    #[test]
    fn test_fingerprint_different_reviewer() {
        let finding = Finding {
            id: "TEST-001".to_string(),
            finding_type: "sql-injection".to_string(),
            title: "SQL Injection".to_string(),
            priority: Priority::P0,
            file: PathBuf::from("src/db.py"),
            line: 42,
            snippet: None,
            description: "Bad".to_string(),
            remediation: "Fix it".to_string(),
            acceptance_criteria: vec![],
            references: vec![],
        };

        let fp1 = finding.fingerprint("security-python");
        let fp2 = finding.fingerprint("security-swift");

        assert_ne!(fp1, fp2);
    }

    #[test]
    fn test_normalize_snippet_whitespace() {
        let finding = Finding {
            id: "TEST-001".to_string(),
            finding_type: "test".to_string(),
            title: "Test".to_string(),
            priority: Priority::P1,
            file: PathBuf::from("test.py"),
            line: 1,
            snippet: Some("  foo   bar\n  baz  ".to_string()),
            description: "".to_string(),
            remediation: "".to_string(),
            acceptance_criteria: vec![],
            references: vec![],
        };

        assert_eq!(finding.normalize_snippet(), "foo bar baz");
    }
}
