//! TUI command - launches the interactive terminal UI

use anyhow::{Context, Result};
use std::path::Path;

use super::TuiArgs;
use crate::parser::Finding;
use crate::planner::UnifiedPlan;
use crate::tui::{init_local_offset, run_tui, TuiConfig};

pub fn execute(args: TuiArgs) -> Result<()> {
    // Initialize timezone offset before entering TUI (must be single-threaded)
    init_local_offset();

    let mut config = TuiConfig {
        start_in_plan_mode: args.plan_mode,
        ..Default::default()
    };

    // Load findings
    if let Some(findings_path) = &args.findings {
        config.findings = load_findings(findings_path)?;
        config.findings_path = Some(findings_path.clone());
    } else if args.report_dir.exists() {
        // Try to load most recent findings from report dir
        if let Some((findings, path)) = load_latest_findings(&args.report_dir)? {
            config.findings = findings;
            config.findings_path = Some(path);
        }
    }

    // Load plan
    if let Some(plan_path) = &args.plan {
        config.unified_plan = Some(load_plan(plan_path)?);
        config.start_in_plan_mode = true;
    } else if let Some(plan) = load_latest_plan()? {
        // Auto-discover from .agentic/plans/
        config.unified_plan = Some(plan);
        config.start_in_plan_mode = true;
    }

    // If we have findings but no plan, default to findings view
    if !config.findings.is_empty() && config.unified_plan.is_none() {
        config.start_in_plan_mode = false;
    }

    run_tui(config)
}

fn load_findings(path: &Path) -> Result<Vec<Finding>> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read findings from {}", path.display()))?;

    serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse findings from {}", path.display()))
}

fn load_plan(path: &Path) -> Result<UnifiedPlan> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read plan from {}", path.display()))?;

    serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse plan from {}", path.display()))
}

/// Auto-discover the most recent plan from .agentic/plans/
fn load_latest_plan() -> Result<Option<UnifiedPlan>> {
    let plans_dir = Path::new(".agentic/plans");
    if !plans_dir.is_dir() {
        return Ok(None);
    }

    // Find most recent plan.json by scanning subdirectories
    let mut latest: Option<(std::fs::Metadata, std::path::PathBuf)> = None;

    for entry in std::fs::read_dir(plans_dir)? {
        let entry = entry?;
        let path = entry.path();

        // Each subdirectory is {date}-{name}/, look for plan.json inside
        if path.is_dir() {
            let plan_path = path.join("plan.json");
            if plan_path.exists() {
                if let Ok(meta) = plan_path.metadata() {
                    if let Ok(modified) = meta.modified() {
                        let dominated = latest.as_ref().map_or(false, |(m, _)| {
                            m.modified().map_or(false, |t| modified > t)
                        });
                        if latest.is_none() || dominated {
                            latest = Some((meta, plan_path));
                        }
                    }
                }
            }
        }
    }

    if let Some((_, path)) = latest {
        let plan = load_plan(&path)?;
        Ok(Some(plan))
    } else {
        Ok(None)
    }
}

fn load_latest_findings(report_dir: &Path) -> Result<Option<(Vec<Finding>, std::path::PathBuf)>> {
    if !report_dir.is_dir() {
        return Ok(None);
    }

    // Find most recent .findings.json file
    let mut latest: Option<(std::fs::Metadata, std::path::PathBuf)> = None;

    for entry in std::fs::read_dir(report_dir)? {
        let entry = entry?;
        let path = entry.path();

        // Check subdirectories (date-based) and root
        if path.is_dir() {
            if let Ok(subentries) = std::fs::read_dir(&path) {
                for subentry in subentries.flatten() {
                    let subpath = subentry.path();
                    if subpath.to_string_lossy().ends_with(".findings.json") {
                        if let Ok(meta) = subpath.metadata() {
                            if let Ok(modified) = meta.modified() {
                                let dominated = latest.as_ref().map_or(false, |(m, _)| {
                                    m.modified().map_or(false, |t| modified > t)
                                });
                                if latest.is_none() || dominated {
                                    latest = Some((meta, subpath));
                                }
                            }
                        }
                    }
                }
            }
        } else if path.to_string_lossy().ends_with(".findings.json") {
            if let Ok(meta) = path.metadata() {
                if let Ok(modified) = meta.modified() {
                    let dominated = latest.as_ref().map_or(false, |(m, _)| {
                        m.modified().map_or(false, |t| modified > t)
                    });
                    if latest.is_none() || dominated {
                        latest = Some((meta, path));
                    }
                }
            }
        }
    }

    if let Some((_, path)) = latest {
        let findings = load_findings(&path)?;
        Ok(Some((findings, path)))
    } else {
        Ok(None)
    }
}
