use crate::cli::RunArgs;
use crate::config::Config;
use crate::output::write_summary;
use crate::postprocess::run_postprocess;
use crate::runner::{Orchestrator, RunOptions};
use crate::state::State;
use chrono::Local;
use tracing::{error, info, warn};

pub async fn execute(args: RunArgs) -> anyhow::Result<()> {
    // Load and validate config
    info!("Loading config from {:?}", args.config);
    let mut config = Config::load(&args.config)?;

    // Apply CLI overrides
    if let Some(concurrency) = args.concurrency {
        config.concurrency = concurrency;
    }
    if let Some(report_dir) = args.report_dir {
        config.report_dir = report_dir;
    }
    if args.dry_run {
        config.dry_run = true;
    }

    // Load state to check for recent runs
    let mut state = State::load(&config.target);

    // Build run options
    let options = RunOptions {
        reviewer_filter: args.reviewers,
        scope_filter: args.scopes,
        diff_base: args.diff_base.or(config.diff_base.clone()),
        dry_run: config.dry_run,
        force: args.force,
    };

    // Validate config
    config.validate()?;

    // Check which reviewers already ran today (unless --force)
    if !args.force {
        let mut skipped_today = Vec::new();
        for reviewer in &config.reviewers {
            if reviewer.enabled && state.ran_today(&reviewer.id) {
                skipped_today.push(reviewer.id.clone());
            }
        }
        if !skipped_today.is_empty() {
            info!(
                "Skipping {} reviewers that already ran today: {:?}",
                skipped_today.len(),
                skipped_today
            );
            info!("Use --force to re-run them");
        }
    }

    if config.dry_run {
        info!("DRY RUN - no provider calls will be made");
        print_execution_plan(&config, &options, &state, args.force);
        return Ok(());
    }

    // Create dated report directory (reports/YYYY-MM-DD/)
    let date_str = Local::now().format("%Y-%m-%d").to_string();
    let report_dir = config.report_dir.join(&date_str);

    // Create orchestrator and run (reports written as each reviewer completes)
    info!("Reports will be written to {:?}", report_dir);
    let orchestrator = Orchestrator::new(config.clone())?;
    let report = orchestrator.run(&options, &state, &report_dir).await?;

    // Update state with run results
    for result in &report.reviewer_results {
        // Only record successful runs; allow failed/timeouts to rerun without --force
        if matches!(result.status, crate::runner::ReviewerStatus::Completed) {
            state.record_run(&result.reviewer_id, result.findings.len());
        }
    }

    // Save state
    if let Err(e) = state.save(&config.target) {
        warn!("Failed to save state: {}", e);
    }

    // Write summary artifacts
    if let Err(e) = write_summary(&report_dir, &report, &config.target) {
        warn!("Failed to write summary: {}", e);
    }

    // Optional postprocess step (reducer / clustering)
    match run_postprocess(&config, &report_dir).await {
        Ok(Some(result)) => {
            info!(
                "Postprocess: {} -> {} findings",
                result.original_count, result.reduced_count
            );
        }
        Ok(None) => {
            // Postprocess disabled or skipped
        }
        Err(e) => {
            warn!("Postprocess step failed: {}", e);
        }
    }

    // Summary
    let totals = report.totals();
    info!(
        "Completed in {:.1}s: {} p0, {} p1, {} p2 findings across {} reviewers",
        report.total_duration.as_secs_f64(),
        totals.p0,
        totals.p1,
        totals.p2,
        report.reviewer_results.len()
    );

    // Create GitHub issues if requested
    if args.create_issues {
        let total_findings = totals.p0 + totals.p1 + totals.p2;
        if total_findings > 0 {
            info!("Creating GitHub issues...");
            let issue_args = crate::cli::IssueArgs {
                files: vec![],
                report_dir: report_dir.clone(),
                config: args.config.clone(),
                dry_run: false,
                force: false,
                repo: None,
            };
            if let Err(e) = crate::cli::issue::execute(issue_args).await {
                error!("Failed to create issues: {}", e);
            }
        } else {
            info!("No findings to create issues for");
        }
    }

    // Exit with error if critical findings and flag set
    if args.fail_on_critical && totals.p0 > 0 {
        error!("Exiting with error: {} critical (p0) findings", totals.p0);
        std::process::exit(1);
    }

    Ok(())
}

fn print_execution_plan(config: &Config, options: &RunOptions, state: &State, force: bool) {
    println!("\n=== Execution Plan ===\n");
    println!("Target: {:?}", config.target);
    println!("Concurrency: {}", config.concurrency);
    println!("Report dir: {:?}", config.report_dir);

    if let Some(ref diff_base) = options.diff_base {
        println!("Diff base: {}", diff_base);
    }

    println!("\nReviewers to run:");
    for reviewer in &config.reviewers {
        if !reviewer.enabled {
            continue;
        }
        if let Some(ref filter) = options.reviewer_filter {
            if !filter.contains(&reviewer.id) {
                continue;
            }
        }
        if let Some(ref filter) = options.scope_filter {
            if !reviewer.scopes.iter().any(|s| filter.contains(s)) {
                continue;
            }
        }

        let ran_today = state.ran_today(&reviewer.id);
        let status = if ran_today && !force {
            " [SKIP - already ran today]"
        } else if ran_today && force {
            " [FORCE re-run]"
        } else {
            ""
        };

        println!(
            "  - {} ({:?}) -> scopes: {:?}{}",
            reviewer.id, reviewer.provider, reviewer.scopes, status
        );
    }
    println!();
}
