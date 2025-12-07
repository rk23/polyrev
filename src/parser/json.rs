use super::Finding;
use serde::Deserialize;

/// Try to parse findings from JSON output
pub fn try_parse_json(raw: &str) -> Option<Vec<Finding>> {
    // Claude wraps result in {"result": "...", ...} JSON
    #[derive(Deserialize)]
    struct ClaudeOutput {
        result: String,
    }

    // Try Claude format first
    if let Ok(claude_out) = serde_json::from_str::<ClaudeOutput>(raw) {
        if let Some(findings) = parse_findings_json(&claude_out.result) {
            return Some(findings);
        }
    }

    // Try direct JSON
    parse_findings_json(raw)
}

fn parse_findings_json(s: &str) -> Option<Vec<Finding>> {
    // Try to find JSON in the string (might be wrapped in markdown code blocks)
    let json_str = extract_json(s)?;

    #[derive(Deserialize)]
    struct FindingsWrapper {
        findings: Vec<Finding>,
    }

    match serde_json::from_str::<FindingsWrapper>(&json_str) {
        Ok(wrapper) => Some(wrapper.findings),
        Err(e) => {
            tracing::debug!("Failed to parse findings JSON: {}", e);
            None
        }
    }
}

/// Extract JSON object from a string that might contain markdown code blocks
fn extract_json(s: &str) -> Option<String> {
    // First try: the whole string is valid JSON
    if s.trim().starts_with('{')
        && serde_json::from_str::<serde_json::Value>(s.trim()).is_ok()
    {
        return Some(s.trim().to_string());
    }

    // Second try: extract from markdown code block
    let re = regex::Regex::new(r"```(?:json)?\s*\n?([\s\S]*?)\n?```").ok()?;
    for cap in re.captures_iter(s) {
        let potential_json = cap.get(1)?.as_str().trim();
        if serde_json::from_str::<serde_json::Value>(potential_json).is_ok() {
            return Some(potential_json.to_string());
        }
    }

    // Third try: find JSON object pattern
    let brace_start = s.find('{')?;
    let mut depth = 0;
    let mut end = brace_start;

    for (i, c) in s[brace_start..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = brace_start + i + 1;
                    break;
                }
            }
            _ => {}
        }
    }

    if depth == 0 && end > brace_start {
        let potential_json = &s[brace_start..end];
        if serde_json::from_str::<serde_json::Value>(potential_json).is_ok() {
            return Some(potential_json.to_string());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_direct_json() {
        let json = r#"{"findings": [{"id": "T1", "title": "Test", "priority": "p0", "file": "a.py", "line": 1, "description": "d", "remediation": "r"}]}"#;
        let findings = try_parse_json(json).unwrap();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].id, "T1");
    }

    #[test]
    fn test_parse_claude_wrapped() {
        let json = r#"{"result": "{\"findings\": [{\"id\": \"T1\", \"title\": \"Test\", \"priority\": \"p1\", \"file\": \"a.py\", \"line\": 1, \"description\": \"d\", \"remediation\": \"r\"}]}", "session_id": "abc"}"#;
        let findings = try_parse_json(json).unwrap();
        assert_eq!(findings.len(), 1);
    }

    #[test]
    fn test_parse_markdown_wrapped() {
        let md = r#"
Here are the findings:

```json
{"findings": [{"id": "T1", "title": "Test", "priority": "p2", "file": "a.py", "line": 1, "description": "d", "remediation": "r"}]}
```
"#;
        let findings = try_parse_json(md).unwrap();
        assert_eq!(findings.len(), 1);
    }
}
