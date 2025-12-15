//! Plan reducer: merges fragments from multiple perspectives into a unified plan
//!
//! Similar to postprocess/mod.rs but for plan fragments instead of findings.

use crate::config::Config;
use crate::error::PlannerError;
use crate::provider::{create_runner_for_provider, ProviderConfig};
use std::path::Path;
use std::time::Duration;
use tracing::{debug, info, warn};

use super::types::{
    AcceptanceCriterion, PlanFragment, PlanningResult, PerspectiveStatus, TaskFiles, UnifiedPlan,
    UnifiedTask,
};

/// Result of the reduction step
#[derive(Debug)]
#[allow(dead_code)]
pub struct ReductionResult {
    pub plan: UnifiedPlan,
    pub fragment_count: usize,
    pub task_count_before: usize,
    pub task_count_after: usize,
}

/// Reduce multiple plan fragments into a unified plan
pub async fn reduce_plan(
    config: &Config,
    planning_result: &PlanningResult,
    reducer_prompt_path: &Path,
) -> Result<ReductionResult, PlannerError> {
    // Collect successful fragments
    let fragments: Vec<&PlanFragment> = planning_result
        .fragments
        .iter()
        .filter(|r| r.status == PerspectiveStatus::Completed)
        .filter_map(|r| r.fragment.as_ref())
        .collect();

    if fragments.is_empty() {
        return Err(PlannerError::NoFragmentsToReduce);
    }

    let task_count_before: usize = fragments.iter().map(|f| f.tasks.len()).sum();

    info!(
        "Reducing {} fragments with {} total tasks",
        fragments.len(),
        task_count_before
    );

    // Load reducer prompt
    let prompt_template = load_reducer_prompt(reducer_prompt_path)?;

    // Build full prompt with fragments
    let fragments_json = serde_json::to_string_pretty(&fragments)?;
    let full_prompt = format!(
        "{}\n\n## Input Fragments\n\n```json\n{}\n```",
        prompt_template, fragments_json
    );

    // Invoke CLI
    let timeout = Duration::from_secs(config.timeout_sec);
    let output = invoke_reducer(config, &full_prompt, timeout).await?;

    // Parse output
    let plan = parse_unified_plan(&output)?;

    let task_count_after = plan.tasks.len();

    info!(
        "Reduction complete: {} fragments -> {} tasks (from {} suggested)",
        fragments.len(),
        task_count_after,
        task_count_before
    );

    Ok(ReductionResult {
        plan,
        fragment_count: fragments.len(),
        task_count_before,
        task_count_after,
    })
}

fn load_reducer_prompt(path: &Path) -> Result<String, PlannerError> {
    match std::fs::read_to_string(path) {
        Ok(content) => Ok(content),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            debug!(
                "Reducer prompt '{}' not found, using embedded default",
                path.display()
            );
            Ok(include_str!("../../prompts/plan-reduce.md").to_string())
        }
        Err(e) => Err(PlannerError::Io(std::io::Error::new(
            e.kind(),
            format!("Failed to read reducer prompt '{}': {}", path.display(), e),
        ))),
    }
}

async fn invoke_reducer(
    config: &Config,
    prompt: &str,
    timeout: Duration,
) -> Result<String, PlannerError> {
    let provider_config = ProviderConfig {
        binary: config.providers.claude_cli.binary.clone(),
        model: config.providers.claude_cli.model.clone(),
        tools: config.providers.claude_cli.tools.clone(),
        permission_mode: config.providers.claude_cli.permission_mode.clone(),
    };

    let runner = create_runner_for_provider(crate::config::Provider::ClaudeCli, provider_config);

    debug!("Invoking reducer with {} byte prompt", prompt.len());

    let output = runner.execute(prompt, &[], timeout, None).await?;

    if output.exit_code != 0 {
        return Err(PlannerError::CliExecution(format!(
            "Reducer CLI exited with code {}: {}",
            output.exit_code, output.stderr
        )));
    }

    Ok(output.stdout)
}

fn parse_unified_plan(raw: &str) -> Result<UnifiedPlan, PlannerError> {
    // Claude wraps result in {"result": "...", ...} JSON
    #[derive(serde::Deserialize)]
    struct ClaudeOutput {
        result: String,
    }

    // Try Claude format first
    if let Ok(claude_out) = serde_json::from_str::<ClaudeOutput>(raw) {
        if let Some(plan) = try_parse_plan(&claude_out.result) {
            return Ok(plan);
        }
    }

    // Try direct parse
    if let Some(plan) = try_parse_plan(raw) {
        return Ok(plan);
    }

    Err(PlannerError::ParseOutput(
        "Could not parse unified plan from reducer output".to_string(),
    ))
}

fn try_parse_plan(s: &str) -> Option<UnifiedPlan> {
    let json_str = extract_json(s)?;

    // Try direct parse
    if let Ok(plan) = serde_json::from_str::<UnifiedPlan>(&json_str) {
        return Some(plan);
    }

    // Try with flexible field names
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&json_str) {
        let tasks = value
            .get("tasks")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        let questions = value
            .get("questions")
            .or_else(|| value.get("questions_for_human"))
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        let risks = value
            .get("risks")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        let deferred = value
            .get("deferred")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        let summary = value
            .get("summary")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        return Some(UnifiedPlan {
            tasks,
            questions,
            risks,
            deferred,
            summary,
        });
    }

    debug!(
        "Failed to parse unified plan from: {}...",
        &json_str.chars().take(200).collect::<String>()
    );
    None
}

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

/// Write the plan result to a JSON file
pub fn write_plan(output_path: &Path, plan: &UnifiedPlan) -> Result<(), PlannerError> {
    let parent = output_path.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent)?;

    let json = serde_json::to_string_pretty(plan)?;
    std::fs::write(output_path, json)?;

    info!("Wrote plan to {}", output_path.display());
    Ok(())
}

/// Write raw fragments for debugging
pub fn write_fragments(
    output_dir: &Path,
    planning_result: &PlanningResult,
) -> Result<(), PlannerError> {
    std::fs::create_dir_all(output_dir)?;

    for result in &planning_result.fragments {
        if let Some(ref fragment) = result.fragment {
            let path = output_dir.join(format!("{}.fragment.json", result.perspective_id));
            let json = serde_json::to_string_pretty(fragment)?;
            std::fs::write(&path, json)?;
            debug!("Wrote fragment to {}", path.display());
        }
    }

    Ok(())
}

/// Simplified task format for revision output (YAML)
#[derive(Debug, Clone, serde::Deserialize)]
struct RevisedTask {
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    acceptance_criteria: Vec<String>,
    #[serde(default)]
    files: Vec<String>,
    #[serde(default)]
    depends_on: Vec<String>,
}

/// YAML revision output format
#[derive(Debug, Clone, serde::Deserialize)]
struct RevisionOutput {
    tasks: Vec<RevisedTask>,
    #[serde(default)]
    revision_summary: Option<String>,
}

const MAX_REVISION_RETRIES: usize = 2;

/// Revise a plan based on user answers to questions
pub async fn revise_plan(
    config: &Config,
    plan: &UnifiedPlan,
    answers: &[(String, String)], // (question, answer) pairs
) -> Result<UnifiedPlan, PlannerError> {
    if answers.is_empty() {
        return Ok(plan.clone());
    }

    info!("Revising plan based on {} answered questions", answers.len());

    // Build the revision prompt with simpler task format (YAML)
    let tasks_yaml = plan
        .tasks
        .iter()
        .map(|t| {
            let ac_yaml = if t.acceptance_criteria.is_empty() {
                String::new()
            } else {
                t.acceptance_criteria
                    .iter()
                    .map(|ac| format!("      - \"{}\"\n", ac.criterion.replace('"', "'")))
                    .collect::<String>()
            };
            let files_yaml = if t.files.target.is_empty() {
                String::new()
            } else {
                t.files
                    .target
                    .iter()
                    .map(|f| format!("      - \"{}\"\n", f.display()))
                    .collect::<String>()
            };
            format!(
                "  - title: \"{}\"\n    description: \"{}\"\n    acceptance_criteria:\n{}    files:\n{}",
                t.title.replace('"', "'"),
                t.description.replace('"', "'").replace('\n', " "),
                ac_yaml,
                files_yaml
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let answers_text: String = answers
        .iter()
        .enumerate()
        .map(|(i, (q, a))| format!("{}. Q: {}\n   A: {}", i + 1, q, a))
        .collect::<Vec<_>>()
        .join("\n\n");

    let prompt = format!(
        "{}\n\n## Original Tasks\n\n```yaml\ntasks:\n{}\n```\n\n## User's Answers\n\n{}\n\n## Instructions\n\nRevise ALL tasks to match the user's answers. Output the complete revised task list as YAML.",
        crate::planner::orchestrator::DEFAULT_REVISE_PROMPT,
        tasks_yaml,
        answers_text
    );

    let timeout = std::time::Duration::from_secs(config.timeout_sec);

    // Retry loop
    let mut last_error = None;
    for attempt in 0..=MAX_REVISION_RETRIES {
        if attempt > 0 {
            warn!("Revision attempt {} of {}", attempt + 1, MAX_REVISION_RETRIES + 1);
        }

        let output = invoke_reducer(config, &prompt, timeout).await?;

        match parse_revision_yaml(&output, plan) {
            Ok(revised) => {
                info!(
                    "Revision complete: {} tasks (was {})",
                    revised.tasks.len(),
                    plan.tasks.len()
                );
                return Ok(revised);
            }
            Err(e) => {
                debug!("Revision parse failed (attempt {}): {}", attempt + 1, e);
                last_error = Some(e);
            }
        }
    }

    Err(last_error.unwrap_or_else(|| {
        PlannerError::ParseOutput("Revision failed after retries".to_string())
    }))
}

/// Parse YAML revision output and convert to UnifiedPlan
fn parse_revision_yaml(raw: &str, original: &UnifiedPlan) -> Result<UnifiedPlan, PlannerError> {
    // Extract YAML from output (may be wrapped in markdown code block or Claude JSON)
    let yaml_str = extract_yaml(raw)?;

    let revision: RevisionOutput = serde_yaml::from_str(&yaml_str).map_err(|e| {
        debug!("YAML parse error: {}", e);
        debug!("Input was: {}...", &yaml_str.chars().take(500).collect::<String>());
        PlannerError::ParseOutput(format!("Failed to parse revision YAML: {}", e))
    })?;

    if revision.tasks.is_empty() {
        return Err(PlannerError::ParseOutput(
            "Revision produced no tasks".to_string(),
        ));
    }

    // First pass: assign IDs to all tasks and build title->id mapping
    let mut title_to_id: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let tasks_with_ids: Vec<(RevisedTask, String)> = revision
        .tasks
        .into_iter()
        .enumerate()
        .map(|(i, t)| {
            let original_task = original.tasks.iter().find(|ot| ot.title == t.title);
            let id = original_task
                .map(|ot| ot.id.clone())
                .unwrap_or_else(|| format!("impl-{:03}", i + 1));
            title_to_id.insert(t.title.clone(), id.clone());
            (t, id)
        })
        .collect();

    // Second pass: convert to UnifiedTask, resolving title-based deps to IDs
    let tasks: Vec<UnifiedTask> = tasks_with_ids
        .into_iter()
        .map(|(t, id)| {
            let original_task = original.tasks.iter().find(|ot| ot.title == t.title);

            // Resolve depends_on: titles -> IDs
            let depends_on: Vec<String> = t
                .depends_on
                .into_iter()
                .map(|dep| {
                    // If it's already an ID (impl-XXX), keep it
                    if dep.starts_with("impl-") {
                        dep
                    } else {
                        // Otherwise, look up by title
                        title_to_id.get(&dep).cloned().unwrap_or(dep)
                    }
                })
                .collect();

            UnifiedTask {
                id,
                title: t.title,
                description: t.description,
                files: TaskFiles {
                    target: t.files.into_iter().map(std::path::PathBuf::from).collect(),
                    context: original_task
                        .map(|ot| ot.files.context.clone())
                        .unwrap_or_default(),
                },
                depends_on,
                acceptance_criteria: t
                    .acceptance_criteria
                    .into_iter()
                    .map(|c| AcceptanceCriterion {
                        criterion: c,
                        verification: String::new(),
                    })
                    .collect(),
                perspectives: original_task
                    .map(|ot| ot.perspectives.clone())
                    .unwrap_or_default(),
                workflow: original_task.and_then(|ot| ot.workflow.clone()),
                priority: original_task
                    .map(|ot| ot.priority)
                    .unwrap_or_default(),
            }
        })
        .collect();

    Ok(UnifiedPlan {
        tasks,
        questions: original.questions.clone(),
        risks: original.risks.clone(),
        deferred: original.deferred.clone(),
        summary: revision.revision_summary.or(original.summary.clone()),
    })
}

/// Extract YAML content from raw output (handles code blocks and Claude JSON wrapper)
fn extract_yaml(raw: &str) -> Result<String, PlannerError> {
    // First, try to unwrap Claude's JSON wrapper
    #[derive(serde::Deserialize)]
    struct ClaudeOutput {
        result: String,
    }

    let content = if let Ok(claude_out) = serde_json::from_str::<ClaudeOutput>(raw) {
        claude_out.result
    } else {
        raw.to_string()
    };

    let trimmed = content.trim();

    // Try to extract from markdown code block
    let yaml_regex = regex::Regex::new(r"```(?:yaml)?\s*\n?([\s\S]*?)\n?```")
        .map_err(|e| PlannerError::ParseOutput(format!("Regex error: {}", e)))?;

    if let Some(cap) = yaml_regex.captures(trimmed) {
        if let Some(yaml_match) = cap.get(1) {
            let yaml_str = yaml_match.as_str().trim();
            // Validate it's parseable YAML
            if serde_yaml::from_str::<serde_yaml::Value>(yaml_str).is_ok() {
                return Ok(yaml_str.to_string());
            }
        }
    }

    // Try the whole content as YAML
    if serde_yaml::from_str::<serde_yaml::Value>(trimmed).is_ok() {
        return Ok(trimmed.to_string());
    }

    // Try to find yaml-like content starting with "tasks:"
    if let Some(start) = trimmed.find("tasks:") {
        let yaml_content = &trimmed[start..];
        if serde_yaml::from_str::<serde_yaml::Value>(yaml_content).is_ok() {
            return Ok(yaml_content.to_string());
        }
    }

    Err(PlannerError::ParseOutput(
        "Could not extract YAML from revision output".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_unified_plan() {
        let json = r#"{
            "tasks": [
                {
                    "id": "impl-001",
                    "title": "Add config",
                    "description": "Setup configuration",
                    "depends_on": [],
                    "perspectives": ["architecture"]
                }
            ],
            "questions": [],
            "risks": [],
            "summary": "1 task generated"
        }"#;

        let plan = try_parse_plan(json).unwrap();
        assert_eq!(plan.tasks.len(), 1);
        assert_eq!(plan.tasks[0].id, "impl-001");
    }

    #[test]
    fn test_parse_with_questions_for_human() {
        let json = r#"{
            "tasks": [],
            "questions_for_human": [
                {"question": "Redis or memory?", "raised_by": ["arch", "security"]}
            ],
            "risks": []
        }"#;

        let plan = try_parse_plan(json).unwrap();
        assert_eq!(plan.questions.len(), 1);
    }
}
