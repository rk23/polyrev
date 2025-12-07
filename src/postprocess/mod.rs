//! Postprocess module: aggregates and reduces findings using AI
//!
//! After all reviewers complete, this module:
//! 1. Collects all findings from `*.findings.json` files
//! 2. Invokes the configured CLI to deduplicate and cluster findings
//! 3. Writes the reduced output to `reduced.json`

use crate::config::Config;
use crate::error::PostprocessError;
use crate::parser::Finding;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout as tokio_timeout;
use tracing::{debug, info, warn};

/// A finding with its source reviewer attached
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourcedFinding {
    pub reviewer_id: String,
    pub fingerprint: String,
    #[serde(flatten)]
    pub finding: Finding,
}

/// The output format we expect from the reduction prompt
#[derive(Debug, Deserialize)]
struct ReducedOutput {
    /// Deduplicated/merged findings
    pub findings: Vec<ReducedFinding>,
    /// Clusters of related findings (by fingerprint)
    #[serde(default)]
    pub clusters: Vec<FindingCluster>,
    /// Summary of the reduction
    #[serde(default)]
    pub summary: Option<String>,
}

/// A reduced finding after deduplication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReducedFinding {
    /// Fingerprints of original findings that were merged into this one
    #[serde(default)]
    pub merged_from: Vec<String>,

    #[serde(default)]
    pub id: String,

    #[serde(default, alias = "type")]
    pub finding_type: String,

    #[serde(default)]
    pub title: String,

    #[serde(default)]
    pub priority: String,

    #[serde(default)]
    pub file: PathBuf,

    #[serde(default)]
    pub line: u32,

    #[serde(default)]
    pub description: String,

    #[serde(default)]
    pub remediation: String,

    #[serde(default)]
    pub acceptance_criteria: Vec<String>,

    #[serde(default)]
    pub references: Vec<String>,
}

/// A cluster of related findings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindingCluster {
    /// Human-readable cluster name
    pub name: String,
    /// Fingerprints of findings in this cluster
    pub fingerprints: Vec<String>,
    /// Why these findings are related
    pub rationale: String,
}

/// Final output written to reduced.json
#[derive(Debug, Serialize)]
pub struct PostprocessResult {
    pub original_count: usize,
    pub reduced_count: usize,
    pub clusters: Vec<FindingCluster>,
    pub findings: Vec<ReducedFinding>,
    pub summary: Option<String>,
}

/// Run the postprocessing step
pub async fn run_postprocess(
    config: &Config,
    report_dir: &Path,
) -> Result<Option<PostprocessResult>, PostprocessError> {
    if !config.postprocess.enabled {
        debug!("Postprocess disabled, skipping");
        return Ok(None);
    }

    info!("Starting postprocess step");

    // Collect all findings
    let sourced_findings = collect_findings(report_dir)?;

    if sourced_findings.len() < config.postprocess.min_findings {
        info!(
            "Only {} findings found, below threshold of {} - skipping reduction",
            sourced_findings.len(),
            config.postprocess.min_findings
        );
        // Still write the aggregated findings even if we skip reduction
        let result = PostprocessResult {
            original_count: sourced_findings.len(),
            reduced_count: sourced_findings.len(),
            clusters: vec![],
            findings: sourced_findings
                .into_iter()
                .map(|sf| ReducedFinding {
                    merged_from: vec![sf.fingerprint],
                    id: sf.finding.id,
                    finding_type: sf.finding.finding_type,
                    title: sf.finding.title,
                    priority: sf.finding.priority.to_string(),
                    file: sf.finding.file,
                    line: sf.finding.line,
                    description: sf.finding.description,
                    remediation: sf.finding.remediation,
                    acceptance_criteria: sf.finding.acceptance_criteria,
                    references: sf.finding.references,
                })
                .collect(),
            summary: None,
        };
        write_result(report_dir, &result)?;
        return Ok(Some(result));
    }

    info!(
        "Reducing {} findings using {}",
        sourced_findings.len(),
        config.postprocess.tool
    );

    // Load the reduction prompt
    let prompt_content = std::fs::read_to_string(&config.postprocess.prompt_file).map_err(|e| {
        PostprocessError::Io(std::io::Error::new(
            e.kind(),
            format!(
                "Failed to read prompt file '{}': {}",
                config.postprocess.prompt_file.display(),
                e
            ),
        ))
    })?;

    // Build the full prompt with findings
    let findings_json = serde_json::to_string_pretty(&sourced_findings)?;
    let full_prompt = format!(
        "{}\n\n## Input Findings\n\n```json\n{}\n```",
        prompt_content, findings_json
    );

    // Invoke the CLI
    let timeout = Duration::from_secs(config.postprocess.timeout_sec);
    let output = invoke_cli(config, &full_prompt, timeout).await?;

    // Parse the output
    let reduced = parse_reduced_output(&output)?;

    let result = PostprocessResult {
        original_count: sourced_findings.len(),
        reduced_count: reduced.findings.len(),
        clusters: reduced.clusters,
        findings: reduced.findings,
        summary: reduced.summary,
    };

    info!(
        "Reduction complete: {} -> {} findings ({} clusters)",
        result.original_count,
        result.reduced_count,
        result.clusters.len()
    );

    write_result(report_dir, &result)?;

    Ok(Some(result))
}

/// Collect all findings from *.findings.json files in the report directory
fn collect_findings(report_dir: &Path) -> Result<Vec<SourcedFinding>, PostprocessError> {
    let mut out = Vec::new();
    let mut files_processed = 0;
    let mut files_skipped = 0;

    for path in walk_findings(report_dir)? {
        let reviewer_id = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .trim_end_matches(".findings")
            .to_string();

        let content = std::fs::read_to_string(&path)?;
        let findings: Vec<Finding> = match serde_json::from_str(&content) {
            Ok(f) => f,
            Err(e) => {
                warn!("Failed to parse {:?}: {}", path, e);
                files_skipped += 1;
                continue;
            }
        };

        files_processed += 1;
        for finding in findings {
            let fingerprint = finding.fingerprint(&reviewer_id);
            out.push(SourcedFinding {
                reviewer_id: reviewer_id.clone(),
                fingerprint,
                finding,
            });
        }
    }

    debug!(
        "Collected {} findings from {} files ({} skipped)",
        out.len(),
        files_processed,
        files_skipped
    );

    Ok(out)
}

/// Walk directory recursively to find *.findings.json files
fn walk_findings(dir: &Path) -> Result<Vec<PathBuf>, PostprocessError> {
    let mut files = Vec::new();
    if !dir.exists() {
        return Ok(files);
    }
    walk_dir_recursive(dir, &mut files)?;
    Ok(files)
}

fn walk_dir_recursive(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), PostprocessError> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            walk_dir_recursive(&path, files)?;
        } else if path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.ends_with(".findings.json"))
            .unwrap_or(false)
        {
            files.push(path);
        }
    }
    Ok(())
}

/// Invoke the configured CLI with the reduction prompt
async fn invoke_cli(
    config: &Config,
    prompt: &str,
    timeout: Duration,
) -> Result<String, PostprocessError> {
    let tool = &config.postprocess.tool;

    // Determine provider type from string
    let is_claude = tool == "claude_cli" || tool == "claude";

    let (binary, args) = if is_claude {
        let binary = &config.providers.claude_cli.binary;
        let tools = config.providers.claude_cli.tools.join(",");
        let permission_mode = &config.providers.claude_cli.permission_mode;

        (
            binary.clone(),
            vec![
                "-p".to_string(),
                prompt.to_string(),
                "--output-format".to_string(),
                "json".to_string(),
                "--allowedTools".to_string(),
                tools,
                "--permission-mode".to_string(),
                permission_mode.clone(),
            ],
        )
    } else {
        // codex_cli
        let binary = &config.providers.codex_cli.binary;
        let model = &config.providers.codex_cli.model;

        (
            binary.clone(),
            vec![
                "exec".to_string(),
                "--model".to_string(),
                model.clone(),
                prompt.to_string(),
            ],
        )
    };

    debug!("Invoking {} with {} byte prompt", tool, prompt.len());

    let binary_str = binary.to_string_lossy();
    let mut cmd = if binary_str.contains('/') || binary_str.contains('\\') {
        Command::new(&binary)
    } else {
        Command::new(binary_str.as_ref())
    };

    cmd.current_dir(&config.target);
    cmd.env_remove("ANTHROPIC_API_KEY"); // Use subscription auth
    cmd.args(&args);

    let output = tokio_timeout(timeout, cmd.output())
        .await
        .map_err(|_| PostprocessError::Timeout(timeout))?
        .map_err(PostprocessError::Io)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(PostprocessError::CliExecution(format!(
            "CLI exited with code {}: {}",
            output.status.code().unwrap_or(-1),
            stderr
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(stdout)
}

/// Parse the CLI output to extract reduced findings
fn parse_reduced_output(raw: &str) -> Result<ReducedOutput, PostprocessError> {
    // Claude wraps result in {"result": "...", ...} JSON
    #[derive(Deserialize)]
    struct ClaudeOutput {
        result: String,
    }

    // Try Claude format first
    if let Ok(claude_out) = serde_json::from_str::<ClaudeOutput>(raw) {
        if let Some(reduced) = try_parse_reduced(&claude_out.result) {
            return Ok(reduced);
        }
    }

    // Try direct parse
    if let Some(reduced) = try_parse_reduced(raw) {
        return Ok(reduced);
    }

    Err(PostprocessError::ParseOutput(
        "Could not parse reduced findings from CLI output".to_string(),
    ))
}

fn try_parse_reduced(s: &str) -> Option<ReducedOutput> {
    // Try to find JSON in the string (might be wrapped in markdown code blocks)
    let json_str = extract_json(s)?;

    // Try 1: Expected format {findings: [...], clusters: [...], summary: ...}
    if let Ok(output) = serde_json::from_str::<ReducedOutput>(&json_str) {
        return Some(output);
    }

    // Try 2: Just {findings: [...]} without other fields
    #[derive(Deserialize)]
    struct FindingsOnly {
        findings: Vec<ReducedFinding>,
    }
    if let Ok(fo) = serde_json::from_str::<FindingsOnly>(&json_str) {
        return Some(ReducedOutput {
            findings: fo.findings,
            clusters: vec![],
            summary: None,
        });
    }

    // Try 3: Direct array of findings [...]
    if let Ok(findings) = serde_json::from_str::<Vec<ReducedFinding>>(&json_str) {
        return Some(ReducedOutput {
            findings,
            clusters: vec![],
            summary: None,
        });
    }

    // Try 4: Parse as generic Value and extract findings
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&json_str) {
        if let Some(findings_val) = value.get("findings") {
            if let Ok(findings) = serde_json::from_value::<Vec<ReducedFinding>>(findings_val.clone()) {
                let clusters = value.get("clusters")
                    .and_then(|c| serde_json::from_value::<Vec<FindingCluster>>(c.clone()).ok())
                    .unwrap_or_default();
                let summary = value.get("summary")
                    .and_then(|s| s.as_str())
                    .map(|s| s.to_string());
                return Some(ReducedOutput { findings, clusters, summary });
            }
        }
    }

    debug!("Failed to parse reduced output from: {}...", &json_str.chars().take(200).collect::<String>());
    None
}

/// Extract JSON object or array from a string that might contain markdown code blocks
fn extract_json(s: &str) -> Option<String> {
    let trimmed = s.trim();

    // First try: the whole string is valid JSON (object or array)
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

/// Write the postprocess result to reduced.json
fn write_result(report_dir: &Path, result: &PostprocessResult) -> Result<(), PostprocessError> {
    // Ensure the directory exists
    std::fs::create_dir_all(report_dir)?;

    let out_path = report_dir.join("reduced.json");
    debug!("Writing reduced.json to: {}", out_path.display());

    let json = serde_json::to_string_pretty(result)?;
    std::fs::write(&out_path, json)?;
    info!("Wrote reduced findings to {}", out_path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_direct() {
        let json = r#"{"findings": [], "clusters": []}"#;
        let extracted = extract_json(json).unwrap();
        assert!(extracted.contains("findings"));
    }

    #[test]
    fn test_extract_json_markdown() {
        let md = r#"
Here is the reduced output:

```json
{"findings": [], "clusters": [], "summary": "No duplicates found"}
```
"#;
        let extracted = extract_json(md).unwrap();
        assert!(extracted.contains("summary"));
    }

    #[test]
    fn test_parse_reduced_output() {
        let json = r#"{"findings": [{"merged_from": ["abc123"], "id": "SEC-001", "title": "Test", "priority": "p0", "file": "a.py", "line": 1, "description": "d", "remediation": "r"}], "clusters": []}"#;
        let reduced = try_parse_reduced(json).unwrap();
        assert_eq!(reduced.findings.len(), 1);
        assert_eq!(reduced.findings[0].merged_from, vec!["abc123"]);
    }
}
