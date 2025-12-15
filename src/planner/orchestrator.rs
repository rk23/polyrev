//! Orchestrator for running planning perspectives in parallel
//!
//! Mirrors runner/orchestrator.rs but for planning perspectives instead of reviewers.

use crate::config::Config;
use crate::error::PlannerError;
use crate::provider::{create_runner_for_provider, ProviderConfig};
use futures::stream::{FuturesUnordered, StreamExt};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::time::sleep;
use tracing::{debug, info, warn};

use super::parser::parse_plan_fragment;
use super::types::{Perspective, PerspectiveResult, PerspectiveStatus, PlanningResult};

/// Options for running the planner
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PlanOptions {
    /// The spec/feature to plan for
    pub spec: String,

    /// Only run specific perspectives
    pub perspective_filter: Option<Vec<String>>,

    /// Preview without executing
    pub dry_run: bool,
}

/// Orchestrates parallel planning perspective execution
pub struct PlanOrchestrator {
    config: Config,
    perspectives: Vec<Perspective>,
    semaphore: Arc<Semaphore>,
}

impl PlanOrchestrator {
    pub fn new(config: Config, perspectives: Vec<Perspective>) -> Result<Self, PlannerError> {
        let semaphore = Arc::new(Semaphore::new(config.concurrency));
        Ok(Self {
            config,
            perspectives,
            semaphore,
        })
    }

    pub async fn run(&self, options: &PlanOptions) -> Result<PlanningResult, PlannerError> {
        let start = std::time::Instant::now();

        // Filter perspectives
        let perspectives: Vec<_> = self
            .perspectives
            .iter()
            .filter(|p| p.enabled)
            .filter(|p| {
                options
                    .perspective_filter
                    .as_ref()
                    .map(|f| f.contains(&p.id))
                    .unwrap_or(true)
            })
            .cloned()
            .collect();

        if perspectives.is_empty() {
            return Err(PlannerError::NoPerspectivesMatched);
        }

        info!(
            "Running {} planning perspectives with concurrency {}",
            perspectives.len(),
            self.config.concurrency
        );

        let mut futures = FuturesUnordered::new();
        let launch_delay = Duration::from_millis(self.config.launch_delay_ms);

        for (idx, perspective) in perspectives.into_iter().enumerate() {
            // Small delay between launches
            if idx > 0 && launch_delay > Duration::ZERO {
                sleep(launch_delay).await;
            }

            let permit = self.semaphore.clone().acquire_owned().await?;
            let config = self.config.clone();
            let spec = options.spec.clone();

            futures.push(tokio::spawn(async move {
                let _permit = permit;
                execute_perspective(&config, &perspective, &spec).await
            }));
        }

        let mut results = Vec::new();
        while let Some(result) = futures.next().await {
            match result {
                Ok(Ok(perspective_result)) => {
                    let task_count = perspective_result
                        .fragment
                        .as_ref()
                        .map(|f| f.tasks.len())
                        .unwrap_or(0);
                    let concern_count = perspective_result
                        .fragment
                        .as_ref()
                        .map(|f| f.concerns.len())
                        .unwrap_or(0);

                    info!(
                        "Completed {}: {} tasks, {} concerns ({:?})",
                        perspective_result.perspective_id,
                        task_count,
                        concern_count,
                        perspective_result.status
                    );
                    results.push(perspective_result);
                }
                Ok(Err(e)) => {
                    warn!("Perspective execution failed: {}", e);
                }
                Err(e) => {
                    warn!("Task panicked: {}", e);
                }
            }
        }

        Ok(PlanningResult {
            fragments: results,
            total_duration: start.elapsed(),
        })
    }
}

// Embedded default prompts
const DEFAULT_ARCHITECTURE_PROMPT: &str = include_str!("../../prompts/plan/architecture.md");
const DEFAULT_TESTING_PROMPT: &str = include_str!("../../prompts/plan/testing.md");
const DEFAULT_SECURITY_PROMPT: &str = include_str!("../../prompts/plan/security.md");
const DEFAULT_API_PROMPT: &str = include_str!("../../prompts/plan/api.md");
const DEFAULT_INCREMENTAL_PROMPT: &str = include_str!("../../prompts/plan/incremental.md");
const DEFAULT_SKEPTIC_PROMPT: &str = include_str!("../../prompts/plan/skeptic.md");
const DEFAULT_GENERALIST_PROMPT: &str = include_str!("../../prompts/plan/generalist.md");
const DEFAULT_SELECT_PROMPT: &str = include_str!("../../prompts/plan/select.md");
pub const DEFAULT_REVISE_PROMPT: &str = include_str!("../../prompts/plan/revise.md");

/// Get embedded prompt for a perspective ID
fn get_embedded_prompt(perspective_id: &str) -> Option<&'static str> {
    match perspective_id {
        "architecture" => Some(DEFAULT_ARCHITECTURE_PROMPT),
        "testing" => Some(DEFAULT_TESTING_PROMPT),
        "security" => Some(DEFAULT_SECURITY_PROMPT),
        "api" => Some(DEFAULT_API_PROMPT),
        "incremental" => Some(DEFAULT_INCREMENTAL_PROMPT),
        "skeptic" => Some(DEFAULT_SKEPTIC_PROMPT),
        "generalist" => Some(DEFAULT_GENERALIST_PROMPT),
        _ => None,
    }
}

/// Result of perspective selection
#[derive(Debug, Clone)]
pub struct SelectionResult {
    pub selected: Vec<String>,
    pub reasoning: String,
}

/// Select which perspectives to run based on the task
pub async fn select_perspectives(
    config: &Config,
    perspectives: &[Perspective],
    spec: &str,
    max_count: usize,
) -> Result<SelectionResult, PlannerError> {
    info!("Auto-selecting up to {} perspectives for task", max_count);

    // Build perspective descriptions for the prompt
    let perspective_list: String = perspectives
        .iter()
        .filter(|p| p.enabled)
        .map(|p| format!("- **{}** (id: `{}`): {}", p.name, p.id, p.focus))
        .collect::<Vec<_>>()
        .join("\n");

    // Build the selection prompt
    let prompt = DEFAULT_SELECT_PROMPT
        .replace("{{PERSPECTIVES}}", &perspective_list)
        .replace("{{SPEC}}", spec)
        .replace("{{MAX_COUNT}}", &max_count.to_string());

    // Invoke CLI
    let provider_config = ProviderConfig {
        binary: config.providers.claude_cli.binary.clone(),
        model: config.providers.claude_cli.model.clone(),
        tools: vec![], // No tools needed for selection
        permission_mode: config.providers.claude_cli.permission_mode.clone(),
    };
    let runner = create_runner_for_provider(crate::config::Provider::ClaudeCli, provider_config);

    let timeout = Duration::from_secs(60); // Quick task
    let output = runner.execute(&prompt, &[], timeout, None).await?;

    // Parse the response
    let selection = parse_selection_response(&output.stdout)?;

    info!(
        "Selected {} perspectives: {:?}",
        selection.selected.len(),
        selection.selected
    );
    debug!("Selection reasoning: {}", selection.reasoning);

    Ok(selection)
}

fn parse_selection_response(raw: &str) -> Result<SelectionResult, PlannerError> {
    // Try to extract JSON from the response
    let json_str = extract_json_from_response(raw).ok_or_else(|| {
        PlannerError::ParseOutput("Could not find JSON in selection response".to_string())
    })?;

    #[derive(serde::Deserialize)]
    struct SelectResponse {
        selected: Vec<String>,
        #[serde(default)]
        reasoning: String,
    }

    let parsed: SelectResponse = serde_json::from_str(&json_str).map_err(|e| {
        PlannerError::ParseOutput(format!("Failed to parse selection JSON: {}", e))
    })?;

    Ok(SelectionResult {
        selected: parsed.selected,
        reasoning: parsed.reasoning,
    })
}

fn extract_json_from_response(s: &str) -> Option<String> {
    let trimmed = s.trim();

    // Check for Claude's result wrapper
    if let Ok(claude_out) = serde_json::from_str::<serde_json::Value>(trimmed) {
        if let Some(result) = claude_out.get("result") {
            if let Some(result_str) = result.as_str() {
                return extract_json_from_response(result_str);
            }
        }
        // If it's already valid JSON with 'selected', use it
        if claude_out.get("selected").is_some() {
            return Some(trimmed.to_string());
        }
    }

    // Try to find JSON in markdown code block
    let re = regex::Regex::new(r"```(?:json)?\s*\n?([\s\S]*?)\n?```").ok()?;
    for cap in re.captures_iter(s) {
        let potential_json = cap.get(1)?.as_str().trim();
        if serde_json::from_str::<serde_json::Value>(potential_json).is_ok() {
            return Some(potential_json.to_string());
        }
    }

    // Try to find a JSON object
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

/// Execute a single planning perspective
async fn execute_perspective(
    config: &Config,
    perspective: &Perspective,
    spec: &str,
) -> Result<PerspectiveResult, PlannerError> {
    let start = std::time::Instant::now();

    // Load prompt template - try file first, fall back to embedded
    let prompt_path = if perspective.prompt_file.is_absolute() {
        perspective.prompt_file.clone()
    } else {
        config.target.join(&perspective.prompt_file)
    };

    let prompt_template = match std::fs::read_to_string(&prompt_path) {
        Ok(p) => p,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // Try embedded prompt
            match get_embedded_prompt(&perspective.id) {
                Some(embedded) => {
                    debug!(
                        "Using embedded prompt for perspective '{}'",
                        perspective.id
                    );
                    embedded.to_string()
                }
                None => {
                    return Ok(PerspectiveResult {
                        perspective_id: perspective.id.clone(),
                        perspective_name: perspective.name.clone(),
                        status: PerspectiveStatus::Failed {
                            error: format!(
                                "Prompt file not found ({}) and no embedded default for '{}'",
                                prompt_path.display(),
                                perspective.id
                            ),
                        },
                        fragment: None,
                        duration: start.elapsed(),
                    });
                }
            }
        }
        Err(e) => {
            return Ok(PerspectiveResult {
                perspective_id: perspective.id.clone(),
                perspective_name: perspective.name.clone(),
                status: PerspectiveStatus::Failed {
                    error: format!(
                        "Failed to read prompt file ({}): {}",
                        prompt_path.display(),
                        e
                    ),
                },
                fragment: None,
                duration: start.elapsed(),
            });
        }
    };

    // Build full prompt with spec
    let full_prompt = format!(
        "{}\n\n## Feature/Task to Plan\n\n{}\n\n## Your Perspective: {}\n\nFocus: {}",
        prompt_template, spec, perspective.name, perspective.focus
    );

    // Create runner for claude_cli (planning always uses Claude)
    let provider_config = ProviderConfig {
        binary: config.providers.claude_cli.binary.clone(),
        model: config.providers.claude_cli.model.clone(),
        tools: config.providers.claude_cli.tools.clone(),
        permission_mode: config.providers.claude_cli.permission_mode.clone(),
    };
    let runner = create_runner_for_provider(crate::config::Provider::ClaudeCli, provider_config);

    let timeout = Duration::from_secs(config.timeout_sec);

    // Execute - no files needed for planning (reads codebase via tools)
    let result = runner
        .execute(&full_prompt, &[], timeout, None)
        .await;

    match result {
        Ok(output) => {
            debug!(
                "Perspective {} completed in {:?}",
                perspective.id, output.duration
            );

            // Parse the output into a PlanFragment
            match parse_plan_fragment(&output.stdout, &perspective.id) {
                Ok(fragment) => Ok(PerspectiveResult {
                    perspective_id: perspective.id.clone(),
                    perspective_name: perspective.name.clone(),
                    status: PerspectiveStatus::Completed,
                    fragment: Some(fragment),
                    duration: start.elapsed(),
                }),
                Err(e) => Ok(PerspectiveResult {
                    perspective_id: perspective.id.clone(),
                    perspective_name: perspective.name.clone(),
                    status: PerspectiveStatus::Failed {
                        error: format!("Failed to parse output: {}", e),
                    },
                    fragment: None,
                    duration: start.elapsed(),
                }),
            }
        }
        Err(e) => Ok(PerspectiveResult {
            perspective_id: perspective.id.clone(),
            perspective_name: perspective.name.clone(),
            status: PerspectiveStatus::Failed {
                error: e.to_string(),
            },
            fragment: None,
            duration: start.elapsed(),
        }),
    }
}
