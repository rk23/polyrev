use super::Finding;
use crate::config::Priority;
use regex::Regex;
use std::path::PathBuf;

/// Try to parse findings from a markdown table
/// Expected format: | file | line | severity | type | issue | recommendation |
pub fn try_parse_markdown_table(
    raw: &str,
    reviewer_id: &str,
    default_priority: Priority,
) -> Option<Vec<Finding>> {
    // Match table rows: | file | line | severity | type | issue | recommendation |
    let table_re = Regex::new(
        r"(?m)^\|\s*([^|]+)\s*\|\s*(\d+)\s*\|\s*(p[012]|high|medium|low|critical)\s*\|\s*([^|]+)\s*\|\s*([^|]+)\s*\|\s*([^|]+)\s*\|"
    ).ok()?;

    let mut findings = Vec::new();

    for caps in table_re.captures_iter(raw) {
        let file = caps.get(1)?.as_str().trim();
        let line: u32 = caps.get(2)?.as_str().trim().parse().ok()?;
        let severity_str = caps.get(3)?.as_str().trim().to_lowercase();
        let finding_type = caps.get(4)?.as_str().trim();
        let issue = caps.get(5)?.as_str().trim();
        let recommendation = caps.get(6)?.as_str().trim();

        // Skip header row
        if file.to_lowercase() == "file" {
            continue;
        }

        let priority = match severity_str.as_str() {
            "p0" | "critical" | "high" => Priority::P0,
            "p1" | "medium" => Priority::P1,
            "p2" | "low" => Priority::P2,
            _ => default_priority,
        };

        findings.push(Finding {
            id: format!("{}-{}", reviewer_id.to_uppercase(), findings.len() + 1),
            finding_type: finding_type.to_string(),
            title: issue.to_string(),
            priority,
            file: PathBuf::from(file),
            line,
            snippet: None,
            description: issue.to_string(),
            remediation: recommendation.to_string(),
            acceptance_criteria: Vec::new(),
            references: Vec::new(),
        });
    }

    if findings.is_empty() {
        None
    } else {
        Some(findings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_markdown_table() {
        let md = r#"
# Findings

| file | line | severity | type | issue | recommendation |
|------|------|----------|------|-------|----------------|
| src/db.py | 42 | p0 | sql-injection | SQL Injection | Use parameterized queries |
| src/auth.py | 15 | p1 | hardcoded-secret | Hardcoded API key | Use env vars |
"#;

        let findings = try_parse_markdown_table(md, "security-python", Priority::P1).unwrap();
        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].file, PathBuf::from("src/db.py"));
        assert_eq!(findings[0].line, 42);
        assert_eq!(findings[0].priority, Priority::P0);
        assert_eq!(findings[1].priority, Priority::P1);
    }

    #[test]
    fn test_parse_empty_table() {
        let md = "No findings here";
        let findings = try_parse_markdown_table(md, "test", Priority::P1);
        assert!(findings.is_none());
    }
}
