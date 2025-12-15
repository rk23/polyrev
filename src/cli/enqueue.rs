//! CLI handler for the `enqueue` subcommand
//!
//! Enqueues tasks from a plan.json to tandem via Claude + MCP.

use crate::cli::EnqueueArgs;
use crate::config::Config;
use anyhow::{Context, Result};
use serde_json::json;
use std::process::{Command, Stdio};

/// Task from plan.json
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
struct UnifiedTask {
    id: String,
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    depends_on: Vec<String>,
    #[serde(default)]
    files: TaskFiles,
    #[serde(default)]
    acceptance_criteria: Vec<AcceptanceCriterion>,
    #[serde(default)]
    perspectives: Vec<String>,
}

#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
struct TaskFiles {
    #[serde(default)]
    target: Vec<std::path::PathBuf>,
    #[serde(default)]
    context: Vec<std::path::PathBuf>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
struct AcceptanceCriterion {
    criterion: String,
    #[serde(default)]
    verification: String,
}

#[derive(Debug, serde::Deserialize)]
struct UnifiedPlan {
    tasks: Vec<UnifiedTask>,
    #[serde(default)]
    questions: Vec<Question>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
struct Question {
    question: String,
    #[serde(default)]
    answer: Option<String>,
}

pub fn execute(args: EnqueueArgs) -> Result<()> {
    // Load config if exists
    let config = if args.config.exists() {
        Config::load(&args.config)?
    } else {
        Config::default()
    };

    // Get claude binary path from config
    let claude_binary = config.providers.claude_cli.binary.clone();

    // Read plan
    let plan_content = std::fs::read_to_string(&args.plan)
        .with_context(|| format!("Failed to read plan: {}", args.plan.display()))?;
    let plan: UnifiedPlan = serde_json::from_str(&plan_content)
        .with_context(|| "Failed to parse plan.json")?;

    if plan.tasks.is_empty() {
        println!("No tasks in plan.");
        return Ok(());
    }

    // Check for unanswered questions
    let unanswered: Vec<_> = plan.questions.iter()
        .filter(|q| q.answer.is_none())
        .collect();

    if !unanswered.is_empty() && !args.force {
        println!("Plan has {} unanswered questions:", unanswered.len());
        for q in &unanswered {
            println!("  - {}", q.question);
        }
        println!("\nRe-run `polyrev plan` to answer, or use --force to enqueue anyway.");
        return Ok(());
    }

    // Build prefix for external IDs
    let prefix = args.prefix.clone().unwrap_or_else(|| {
        args.plan.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("plan")
            .to_string()
    });

    if args.dry_run {
        println!("Would enqueue {} tasks via tandem MCP:", plan.tasks.len());
        for task in &plan.tasks {
            let deps = if task.depends_on.is_empty() {
                String::new()
            } else {
                format!(" (after {})", task.depends_on.join(", "))
            };
            println!("  {}:{} {}{}", prefix, task.id, task.title, deps);
        }
        return Ok(());
    }

    // Build the prompt for Claude to enqueue tasks
    let prompt = build_enqueue_prompt(&plan, &prefix);

    println!("Enqueuing {} tasks via Claude + tandem MCP...", plan.tasks.len());

    // Spawn Claude in headless mode - use configured binary path
    let binary_str = claude_binary.to_string_lossy();
    let mut cmd = if binary_str.contains('/') || binary_str.contains('\\') {
        Command::new(&claude_binary)
    } else {
        Command::new(binary_str.as_ref())
    };

    let child = cmd
        .args(["--print", "--dangerously-skip-permissions", "-p", &prompt])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("Failed to spawn claude CLI at '{}'. Is it installed?", claude_binary.display()))?;

    let output = child.wait_with_output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Claude failed: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check for success indicators in output
    if stdout.contains("enqueue") || stdout.contains("task") {
        println!("✓ Tasks enqueued successfully");
        println!("\nRun `tandem tui` to view the task queue");
    } else {
        println!("Claude output:\n{}", stdout);
    }

    Ok(())
}

fn build_enqueue_prompt(plan: &UnifiedPlan, prefix: &str) -> String {
    let mut prompt = String::new();

    prompt.push_str("You have access to tandem MCP tools. Enqueue the following tasks:\n\n");

    // Include answered questions as context
    let answered: Vec<_> = plan.questions.iter()
        .filter_map(|q| q.answer.as_ref().map(|a| (q.question.as_str(), a.as_str())))
        .collect();

    if !answered.is_empty() {
        prompt.push_str("## Context (answered questions)\n\n");
        for (q, a) in &answered {
            prompt.push_str(&format!("- Q: {} → A: {}\n", q, a));
        }
        prompt.push('\n');
    }

    prompt.push_str("## Tasks to enqueue\n\n");
    prompt.push_str("Use the `mcp__tandem__enqueue` tool for each task.\n\n");

    for task in &plan.tasks {
        let external_id = format!("{}:{}", prefix, task.id);

        // Build payload
        let mut payload = json!({
            "description": task.description,
            "task_id": task.id,
        });

        if !task.files.target.is_empty() {
            payload["files"] = json!({
                "target": task.files.target,
                "context": task.files.context,
            });
        }

        if !task.acceptance_criteria.is_empty() {
            payload["acceptance_criteria"] = json!(task.acceptance_criteria);
        }

        if !answered.is_empty() {
            payload["decisions"] = json!(answered.iter()
                .map(|(q, a)| format!("{}: {}", q, a))
                .collect::<Vec<_>>());
        }

        prompt.push_str(&format!("### Task: {}\n", task.title));
        prompt.push_str(&format!("- external_id: `{}`\n", external_id));
        prompt.push_str(&format!("- title: `{}`\n", task.title));
        prompt.push_str(&format!("- payload: `{}`\n", payload));

        if !task.depends_on.is_empty() {
            let deps: Vec<String> = task.depends_on.iter()
                .map(|d| format!("{}:{}", prefix, d))
                .collect();
            prompt.push_str(&format!("- deps_external_ids: `{:?}`\n", deps));
        }

        if !task.perspectives.is_empty() {
            prompt.push_str(&format!("- tags: `{:?}`\n", task.perspectives));
        }

        prompt.push('\n');
    }

    prompt.push_str("\nEnqueue all tasks now. After enqueuing, confirm the count of tasks created.\n");

    prompt
}
