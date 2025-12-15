//! CLI handler for the `plan` subcommand
//!
//! Runs parallel planning perspectives and reduces to a unified task DAG.

use crate::cli::PlanArgs;
use crate::config::Config;
use crate::planner::{
    reduce_plan, revise_plan, select_perspectives, write_fragments, write_plan, PlanOptions,
    PlanOrchestrator, Perspective, PerspectiveStatus,
};
use chrono::Local;
use std::io::{self, Write};
use std::path::PathBuf;
use tracing::info;

/// Default planning perspectives if none configured
fn default_perspectives() -> Vec<Perspective> {
    vec![
        Perspective {
            id: "architecture".to_string(),
            name: "Architecture".to_string(),
            focus: "system design, module boundaries, data flow, patterns".to_string(),
            prompt_file: PathBuf::from("prompts/plan/architecture.md"),
            enabled: true,
        },
        Perspective {
            id: "testing".to_string(),
            name: "Testing".to_string(),
            focus: "test strategy, edge cases, fixtures, coverage".to_string(),
            prompt_file: PathBuf::from("prompts/plan/testing.md"),
            enabled: true,
        },
        Perspective {
            id: "security".to_string(),
            name: "Security".to_string(),
            focus: "auth, validation, secrets, attack surface".to_string(),
            prompt_file: PathBuf::from("prompts/plan/security.md"),
            enabled: true,
        },
        Perspective {
            id: "api".to_string(),
            name: "API Design".to_string(),
            focus: "interface design, backwards compat, errors, docs".to_string(),
            prompt_file: PathBuf::from("prompts/plan/api.md"),
            enabled: true,
        },
        Perspective {
            id: "incremental".to_string(),
            name: "Incremental Delivery".to_string(),
            focus: "smallest shippable slices, parallel work, dependencies".to_string(),
            prompt_file: PathBuf::from("prompts/plan/incremental.md"),
            enabled: true,
        },
        Perspective {
            id: "skeptic".to_string(),
            name: "Skeptic".to_string(),
            focus: "challenge assumptions, hidden complexity, scope creep, simpler alternatives"
                .to_string(),
            prompt_file: PathBuf::from("prompts/plan/skeptic.md"),
            enabled: true,
        },
        Perspective {
            id: "generalist".to_string(),
            name: "Generalist".to_string(),
            focus: "overall approach, code organization, conventions, edge cases, documentation"
                .to_string(),
            prompt_file: PathBuf::from("prompts/plan/generalist.md"),
            enabled: true,
        },
    ]
}

pub async fn execute(args: PlanArgs) -> anyhow::Result<()> {
    // Load config if it exists, otherwise use defaults
    let config = if args.config.exists() {
        info!("Loading config from {:?}", args.config);
        Config::load(&args.config)?
    } else {
        info!("No config found, using defaults");
        Config::default()
    };

    // Get perspectives from config or use defaults
    let perspectives = if let Some(ref planning) = config.planning {
        planning.perspectives.clone()
    } else {
        default_perspectives()
    };

    // Build spec from args
    let spec = if let Some(ref file) = args.file {
        std::fs::read_to_string(file)?
    } else if let Some(ref issue) = args.issue {
        // TODO: Fetch issue from GitHub
        format!("GitHub Issue #{}", issue)
    } else {
        args.spec.join(" ")
    };

    if spec.trim().is_empty() {
        anyhow::bail!("No spec provided. Use positional args, --file, or --issue");
    }

    info!("Planning: {}", spec.lines().next().unwrap_or(&spec));

    // Determine which perspectives to run
    let perspective_filter = if args.auto_select {
        // Use AI to select the most valuable perspectives (reserve 1 slot for generalist)
        print!("Selecting perspectives... ");
        std::io::stdout().flush()?;

        let max_to_select = args.max_perspectives.saturating_sub(1); // Reserve slot for generalist
        let selection =
            select_perspectives(&config, &perspectives, &spec, max_to_select).await?;

        // Always include generalist + selected perspectives
        let mut selected = selection.selected;
        if !selected.contains(&"generalist".to_string()) {
            selected.push("generalist".to_string());
        }

        // Format selected perspectives nicely
        let selected_names: Vec<_> = selected
            .iter()
            .filter_map(|id| perspectives.iter().find(|p| &p.id == id))
            .map(|p| p.name.as_str())
            .collect();

        println!("{}", selected_names.join(", "));
        if !selection.reasoning.is_empty() {
            println!("  {}", selection.reasoning);
        }
        println!();

        Some(selected)
    } else if let Some(ref filter) = args.perspectives {
        // Use explicit filter, respecting max
        Some(filter.iter().take(args.max_perspectives).cloned().collect())
    } else {
        // No filter - run all enabled (up to max)
        None
    };

    // Build options
    let options = PlanOptions {
        spec: spec.clone(),
        perspective_filter,
        dry_run: args.dry_run,
    };

    if args.dry_run {
        println!("\n=== Plan Preview (Dry Run) ===\n");
        println!("Spec: {}", spec.lines().next().unwrap_or(&spec));
        println!("\nPerspectives to run:");
        let to_run: Vec<_> = perspectives
            .iter()
            .filter(|p| p.enabled)
            .filter(|p| {
                options
                    .perspective_filter
                    .as_ref()
                    .map(|f| f.contains(&p.id))
                    .unwrap_or(true)
            })
            .collect();
        for p in to_run {
            println!("  - {} ({})", p.name, p.focus);
        }
        return Ok(());
    }

    // Create output directory
    let date_str = Local::now().format("%Y-%m-%d").to_string();
    let plan_name = sanitize_plan_name(&spec);
    let output_dir = PathBuf::from(".agentic/plans").join(format!("{}-{}", date_str, plan_name));
    std::fs::create_dir_all(&output_dir)?;

    // Phase 1: Run perspectives
    print!("Running perspectives... ");
    io::stdout().flush()?;

    let orchestrator = PlanOrchestrator::new(config.clone(), perspectives)?;
    let planning_result = orchestrator.run(&options).await?;

    // Count results
    let mut succeeded = 0;
    let mut failed = 0;
    for result in &planning_result.fragments {
        match &result.status {
            PerspectiveStatus::Completed => succeeded += 1,
            PerspectiveStatus::Failed { .. } => failed += 1,
            PerspectiveStatus::Skipped { .. } => {}
        }
    }

    if failed > 0 {
        println!("{} completed, {} failed", succeeded, failed);
    } else {
        println!("{} completed", succeeded);
    }

    // Show details for each perspective
    for result in &planning_result.fragments {
        let (icon, detail) = match &result.status {
            PerspectiveStatus::Completed => {
                let task_count = result.fragment.as_ref().map(|f| f.tasks.len()).unwrap_or(0);
                let concern_count = result
                    .fragment
                    .as_ref()
                    .map(|f| f.concerns.len())
                    .unwrap_or(0);
                (
                    "✓",
                    format!("{} tasks, {} concerns", task_count, concern_count),
                )
            }
            PerspectiveStatus::Failed { error } => {
                // Truncate long errors
                let short_error = if error.len() > 60 {
                    format!("{}...", &error[..57])
                } else {
                    error.clone()
                };
                ("✗", short_error)
            }
            PerspectiveStatus::Skipped { reason } => ("○", reason.clone()),
        };
        println!("  {} {:<20} {}", icon, result.perspective_name, detail);
    }

    if succeeded == 0 {
        anyhow::bail!("All perspectives failed");
    }

    // Write fragments for debugging
    if args.skip_reduce || args.save_fragments {
        write_fragments(&output_dir, &planning_result)?;
        if args.skip_reduce {
            println!("\nFragments saved to {}", output_dir.display());
            return Ok(());
        }
    }

    // Phase 2: Reduce
    print!("\nReducing to unified plan... ");
    io::stdout().flush()?;

    let reducer_prompt = config
        .planning
        .as_ref()
        .and_then(|p| p.reducer_prompt.clone())
        .unwrap_or_else(|| PathBuf::from("prompts/plan/reduce.md"));

    let reduction = reduce_plan(&config, &planning_result, &reducer_prompt).await?;

    println!(
        "{} suggestions → {} tasks",
        reduction.task_count_before, reduction.task_count_after
    );

    // Collect answers to questions
    let mut answers: Vec<String> = Vec::new();
    if !reduction.plan.questions.is_empty() && !args.yes {
        println!("\n┌─ Questions ({}) ────────────────────────────────────────────────────┐", reduction.plan.questions.len());
        println!("│ Answer each question. Type a number to pick an option, or enter text.");
        println!("│ Press Enter to skip, 'q' to quit.");
        println!("└─────────────────────────────────────────────────────────────────────┘\n");

        for (i, q) in reduction.plan.questions.iter().enumerate() {
            println!("{}. {}", i + 1, q.question);
            if !q.context.is_empty() {
                // Wrap context text
                for line in textwrap(&q.context, 70) {
                    println!("   {}", line);
                }
            }
            if !q.options.is_empty() {
                for (j, opt) in q.options.iter().enumerate() {
                    println!("   [{}] {}", j + 1, opt);
                }
            }
            print!("\n   → ");
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let input = input.trim();

            if input.eq_ignore_ascii_case("q") {
                println!("Cancelled.");
                return Ok(());
            }

            let answer = if input.is_empty() {
                "(skipped)".to_string()
            } else if let Ok(num) = input.parse::<usize>() {
                // User picked an option number
                if num > 0 && num <= q.options.len() {
                    q.options[num - 1].clone()
                } else {
                    input.to_string()
                }
            } else {
                input.to_string()
            };

            answers.push(answer.clone());
            println!("   ✓ {}\n", answer);
        }
    }

    // Revise plan based on answers if any were provided
    let revised_plan = if !answers.is_empty() {
        // Build Q&A pairs for revision
        let qa_pairs: Vec<(String, String)> = reduction
            .plan
            .questions
            .iter()
            .zip(answers.iter())
            .filter(|(_, a)| *a != "(skipped)")
            .map(|(q, a)| (q.question.clone(), a.clone()))
            .collect();

        if !qa_pairs.is_empty() {
            print!("Revising plan based on answers... ");
            io::stdout().flush()?;

            match revise_plan(&config, &reduction.plan, &qa_pairs).await {
                Ok(revised) => {
                    println!("done ({} tasks)", revised.tasks.len());
                    revised
                }
                Err(e) => {
                    println!("failed after retries: {}", e);
                    println!("\n  Your answers won't be reflected in the plan.");
                    print!("  Continue anyway? [y/n] ");
                    io::stdout().flush()?;

                    let mut input = String::new();
                    io::stdin().read_line(&mut input)?;
                    if !input.trim().eq_ignore_ascii_case("y") {
                        anyhow::bail!("Revision failed and user chose not to continue");
                    }
                    reduction.plan.clone()
                }
            }
        } else {
            reduction.plan.clone()
        }
    } else {
        reduction.plan.clone()
    };

    // Display risks if any
    if !revised_plan.risks.is_empty() {
        println!("\n┌─ Risks ────────────────────────────────────────────────────────────┐");
        for risk in &revised_plan.risks {
            println!("│ ⚠ {}", risk.description);
            if let Some(ref mitigation) = risk.mitigation {
                println!("│   → {}", mitigation);
            }
        }
        println!("└────────────────────────────────────────────────────────────────────┘");
    }

    // Display plan summary
    println!("\n┌─ Plan ({} tasks) ───────────────────────────────────────────────────┐", revised_plan.tasks.len());
    for task in &revised_plan.tasks {
        let deps = if task.depends_on.is_empty() {
            String::new()
        } else {
            format!(" (after {})", task.depends_on.join(", "))
        };
        println!("│ {} {}{}", task.id, task.title, deps);

        if !task.files.target.is_empty() {
            let files: Vec<_> = task
                .files
                .target
                .iter()
                .map(|p| p.display().to_string())
                .collect();
            let files_str = if files.len() > 3 {
                format!("{}, ... (+{})", files[..3].join(", "), files.len() - 3)
            } else {
                files.join(", ")
            };
            println!("│    → {}", files_str);
        }
    }
    println!("└────────────────────────────────────────────────────────────────────┘");

    // Final approval (unless --yes or already answered questions)
    if !args.yes && answers.is_empty() {
        print!("\nSave plan? [y/n] ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") && !input.trim().eq_ignore_ascii_case("yes") {
            println!("Cancelled.");
            return Ok(());
        }
    }

    // Write plan with answers
    let mut final_plan = revised_plan.clone();
    for (i, answer) in answers.iter().enumerate() {
        if i < final_plan.questions.len() && answer != "(skipped)" {
            final_plan.questions[i].answer = Some(answer.clone());
        }
    }

    let plan_path = output_dir.join("plan.json");
    write_plan(&plan_path, &final_plan)?;

    // Summary
    let ready = final_plan
        .tasks
        .iter()
        .filter(|t| t.depends_on.is_empty())
        .count();
    let blocked = final_plan.tasks.len() - ready;

    println!("\n✓ Plan saved to {}", plan_path.display());
    println!("  {} ready, {} blocked", ready, blocked);

    if !args.no_enqueue {
        println!("\n  To enqueue: polyrev enqueue --plan {}", plan_path.display());
    }

    Ok(())
}

/// Sanitize spec into a valid directory name
fn sanitize_plan_name(spec: &str) -> String {
    let first_line = spec.lines().next().unwrap_or(spec);
    first_line
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-')
        .collect::<String>()
        .split_whitespace()
        .take(5)
        .collect::<Vec<_>>()
        .join("-")
        .to_lowercase()
}

/// Simple text wrapping for display
fn textwrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        if current_line.is_empty() {
            current_line = word.to_string();
        } else if current_line.len() + 1 + word.len() <= width {
            current_line.push(' ');
            current_line.push_str(word);
        } else {
            lines.push(current_line);
            current_line = word.to_string();
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}
