//! Parser for plan fragment output from planning perspectives

use crate::error::PlannerError;
use tracing::debug;

use super::types::PlanFragment;

/// Parse the raw output from a planning perspective into a PlanFragment
pub fn parse_plan_fragment(raw: &str, perspective_id: &str) -> Result<PlanFragment, PlannerError> {
    // Claude wraps result in {"result": "...", ...} JSON
    #[derive(serde::Deserialize)]
    struct ClaudeOutput {
        result: String,
    }

    // Try Claude format first
    if let Ok(claude_out) = serde_json::from_str::<ClaudeOutput>(raw) {
        if let Some(fragment) = try_parse_fragment(&claude_out.result, perspective_id) {
            return Ok(fragment);
        }
    }

    // Try direct parse
    if let Some(fragment) = try_parse_fragment(raw, perspective_id) {
        return Ok(fragment);
    }

    Err(PlannerError::ParseOutput(format!(
        "Could not parse plan fragment from perspective {} output",
        perspective_id
    )))
}

fn try_parse_fragment(s: &str, perspective_id: &str) -> Option<PlanFragment> {
    // Try to find JSON in the string
    let json_str = extract_json(s)?;

    // Try parsing as PlanFragment directly
    if let Ok(mut fragment) = serde_json::from_str::<PlanFragment>(&json_str) {
        // Ensure perspective is set
        if fragment.perspective.is_empty() {
            fragment.perspective = perspective_id.to_string();
        }
        return Some(fragment);
    }

    // Try parsing as a wrapper object with different field names
    #[derive(serde::Deserialize)]
    struct AltFragment {
        #[serde(default)]
        perspective: String,
        #[serde(default)]
        summary: String,
        #[serde(default, alias = "proposed_tasks")]
        tasks: Vec<serde_json::Value>,
        #[serde(default, alias = "identified_concerns")]
        concerns: Vec<serde_json::Value>,
        #[serde(default, alias = "open_questions")]
        questions: Vec<serde_json::Value>,
    }

    if let Ok(alt) = serde_json::from_str::<AltFragment>(&json_str) {
        // Convert to proper types
        let tasks = alt
            .tasks
            .into_iter()
            .filter_map(|v| serde_json::from_value(v).ok())
            .collect();
        let concerns = alt
            .concerns
            .into_iter()
            .filter_map(|v| serde_json::from_value(v).ok())
            .collect();
        let questions = alt
            .questions
            .into_iter()
            .filter_map(|v| serde_json::from_value(v).ok())
            .collect();

        return Some(PlanFragment {
            perspective: if alt.perspective.is_empty() {
                perspective_id.to_string()
            } else {
                alt.perspective
            },
            summary: alt.summary,
            tasks,
            concerns,
            questions,
        });
    }

    // Try parsing as generic Value and extract fields
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&json_str) {
        let tasks = value
            .get("tasks")
            .or_else(|| value.get("proposed_tasks"))
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        let concerns = value
            .get("concerns")
            .or_else(|| value.get("identified_concerns"))
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        let questions = value
            .get("questions")
            .or_else(|| value.get("open_questions"))
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        let summary = value
            .get("summary")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let perspective = value
            .get("perspective")
            .and_then(|v| v.as_str())
            .unwrap_or(perspective_id)
            .to_string();

        return Some(PlanFragment {
            perspective,
            summary,
            tasks,
            concerns,
            questions,
        });
    }

    debug!(
        "Failed to parse plan fragment from: {}...",
        &json_str.chars().take(200).collect::<String>()
    );
    None
}

/// Extract JSON object or array from a string that might contain markdown code blocks
fn extract_json(s: &str) -> Option<String> {
    let trimmed = s.trim();

    // First try: the whole string is valid JSON
    if (trimmed.starts_with('{') || trimmed.starts_with('['))
        && serde_json::from_str::<serde_json::Value>(trimmed).is_ok()
    {
        return Some(trimmed.to_string());
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
        let json = r#"{
            "perspective": "architecture",
            "summary": "Modular design needed",
            "tasks": [
                {"title": "Add config", "rationale": "Config first"}
            ],
            "concerns": [],
            "questions": []
        }"#;

        let fragment = parse_plan_fragment(json, "architecture").unwrap();
        assert_eq!(fragment.perspective, "architecture");
        assert_eq!(fragment.tasks.len(), 1);
    }

    #[test]
    fn test_parse_markdown_wrapped() {
        let md = r#"
Here's my analysis:

```json
{
    "perspective": "security",
    "summary": "Several security considerations",
    "tasks": [{"title": "Add CSRF protection", "rationale": "Prevent attacks"}],
    "concerns": [{"description": "Token storage", "severity": "high"}],
    "questions": []
}
```
"#;

        let fragment = parse_plan_fragment(md, "security").unwrap();
        assert_eq!(fragment.perspective, "security");
        assert_eq!(fragment.tasks.len(), 1);
        assert_eq!(fragment.concerns.len(), 1);
    }

    #[test]
    fn test_parse_with_missing_perspective() {
        let json = r#"{
            "summary": "Test",
            "tasks": [],
            "concerns": [],
            "questions": []
        }"#;

        let fragment = parse_plan_fragment(json, "testing").unwrap();
        assert_eq!(fragment.perspective, "testing");
    }
}
