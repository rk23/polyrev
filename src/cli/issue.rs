use crate::cli::IssueArgs;
use crate::config::Config;
use crate::github::{IssueCreator, IssueResult};
use std::path::Path;
use tracing::{info, warn};

pub async fn execute(args: IssueArgs) -> anyhow::Result<()> {
    // Load config (for GitHub settings)
    let config = Config::load(&args.config)?;

    // Determine repo - CLI arg takes precedence, then config
    let repo = args.repo.clone().or_else(|| config.github.repo.clone());
    if repo.is_none() && !args.dry_run {
        anyhow::bail!(
            "No GitHub repository specified. Use --repo owner/repo or set github.repo in config"
        );
    }

    // Load findings from files or scan directory
    let findings = if !args.files.is_empty() {
        // Load specific files
        info!("Loading findings from {} specified files", args.files.len());
        load_findings_from_files(&args.files)?
    } else {
        // Check for reduced.json first (postprocessed findings)
        let reduced_path = args.report_dir.join("reduced.json");
        if reduced_path.exists() {
            info!("Loading reduced findings from {:?}", reduced_path);
            load_reduced_findings(&reduced_path)?
        } else {
            // Fall back to scanning for raw .findings.json files
            info!("Scanning {:?} for findings", args.report_dir);
            scan_findings_dir(&args.report_dir)?
        }
    };

    if findings.is_empty() {
        info!("No findings to create issues for");
        return Ok(());
    }

    info!("Found {} findings to process", findings.len());

    if args.dry_run {
        info!("DRY RUN - previewing issues:");
        for (reviewer_id, finding) in &findings {
            println!(
                "  [{}] {} - {}:{}",
                finding.priority,
                finding.title,
                finding.file.display(),
                finding.line
            );
            println!("    Reviewer: {}", reviewer_id);
            println!("    Fingerprint: {}", finding.fingerprint(reviewer_id));
            println!();
        }
        return Ok(());
    }

    let github_cfg = &config.github;

    if !github_cfg.enabled && !args.dry_run {
        anyhow::bail!("GitHub issue creation is disabled in config. Set github.enabled: true");
    }

    // Create issue creator using config github settings
    let creator = IssueCreator::new(
        repo,
        github_cfg.dedupe && !args.force,
        github_cfg.dedupe_action,
        if github_cfg.labels.is_empty() {
            vec!["polyrev".to_string(), "automated-review".to_string()]
        } else {
            github_cfg.labels.clone()
        },
        github_cfg.assignees.clone(),
        github_cfg.auto_fix.clone(),
        config.providers.claude_cli.model.clone(),
    )?;

    let mut created = 0;
    let mut skipped = 0;
    let mut errors = 0;
    let mut agents_triggered = 0;

    for (reviewer_id, finding) in &findings {
        match creator.create_or_update(finding, reviewer_id).await {
            Ok(IssueResult::Created { url, agent_triggered }) => {
                if agent_triggered {
                    info!("Created: {} -> {} (triggered @{})", finding.title, url, github_cfg.auto_fix.agent);
                    agents_triggered += 1;
                } else {
                    info!("Created: {} -> {}", finding.title, url);
                }
                created += 1;
            }
            Ok(IssueResult::Skipped { issue_number }) => {
                info!(
                    "Skipped (duplicate): {} -> #{}",
                    finding.title, issue_number
                );
                skipped += 1;
            }
            Ok(IssueResult::Commented { issue_number }) => {
                info!("Commented: {} -> #{}", finding.title, issue_number);
                created += 1;
            }
            Ok(IssueResult::Reopened { issue_number }) => {
                info!("Reopened: {} -> #{}", finding.title, issue_number);
                created += 1;
            }
            Err(e) => {
                warn!("Failed to create issue for {}: {}", finding.title, e);
                errors += 1;
            }
        }
    }

    if agents_triggered > 0 {
        info!(
            "Done: {} created, {} skipped, {} errors, {} agents triggered",
            created, skipped, errors, agents_triggered
        );
    } else {
        info!(
            "Done: {} created, {} skipped, {} errors",
            created, skipped, errors
        );
    }

    if errors > 0 {
        std::process::exit(1);
    }

    Ok(())
}

/// Load findings from specific files
fn load_findings_from_files(
    files: &[std::path::PathBuf],
) -> anyhow::Result<Vec<(String, crate::parser::Finding)>> {
    let mut all_findings = Vec::new();

    for path in files {
        if !path.exists() {
            warn!("File not found: {:?}", path);
            continue;
        }

        // Extract reviewer ID from filename (e.g., "api-contract.findings.json" -> "api-contract")
        let reviewer_id = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .trim_end_matches(".findings")
            .to_string();

        let findings: Vec<crate::parser::Finding> =
            serde_json::from_reader(std::fs::File::open(path)?)?;

        info!("Loaded {} findings from {:?}", findings.len(), path);
        for finding in findings {
            all_findings.push((reviewer_id.clone(), finding));
        }
    }

    Ok(all_findings)
}

/// Scan directory recursively for .findings.json files
fn scan_findings_dir(dir: &Path) -> anyhow::Result<Vec<(String, crate::parser::Finding)>> {
    let mut all_findings = Vec::new();

    if !dir.exists() {
        anyhow::bail!("Report directory not found: {:?}", dir);
    }

    // Walk directory recursively to find .findings.json files
    for entry in walkdir(dir)? {
        let path = entry;
        if path.extension().and_then(|e| e.to_str()) == Some("json")
            && path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.ends_with(".findings.json"))
                .unwrap_or(false)
        {
            // Extract reviewer ID from filename
            let reviewer_id = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .trim_end_matches(".findings")
                .to_string();

            match std::fs::File::open(&path)
                .map_err(anyhow::Error::from)
                .and_then(|f| serde_json::from_reader(f).map_err(anyhow::Error::from))
            {
                Ok(findings) => {
                    let findings: Vec<crate::parser::Finding> = findings;
                    info!("Loaded {} findings from {:?}", findings.len(), path);
                    for finding in findings {
                        all_findings.push((reviewer_id.clone(), finding));
                    }
                }
                Err(e) => {
                    warn!("Failed to load {:?}: {}", path, e);
                }
            }
        }
    }

    Ok(all_findings)
}

/// Simple directory walker
fn walkdir(dir: &Path) -> anyhow::Result<Vec<std::path::PathBuf>> {
    let mut results = Vec::new();

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            results.extend(walkdir(&path)?);
        } else {
            results.push(path);
        }
    }

    Ok(results)
}

/// Load findings from reduced.json (postprocessed output)
fn load_reduced_findings(
    path: &Path,
) -> anyhow::Result<Vec<(String, crate::parser::Finding)>> {
    use crate::config::Priority;

    #[derive(serde::Deserialize)]
    struct ReducedOutput {
        findings: Vec<ReducedFinding>,
    }

    #[derive(serde::Deserialize)]
    struct ReducedFinding {
        #[serde(default)]
        id: String,
        #[serde(default, alias = "type")]
        finding_type: String,
        #[serde(default)]
        title: String,
        #[serde(default)]
        priority: String,
        #[serde(default)]
        file: std::path::PathBuf,
        #[serde(default)]
        line: u32,
        #[serde(default)]
        snippet: Option<String>,
        #[serde(default)]
        description: String,
        #[serde(default)]
        remediation: String,
        #[serde(default)]
        acceptance_criteria: Vec<String>,
        #[serde(default)]
        references: Vec<String>,
        #[serde(default)]
        model: Option<String>,
    }

    let content = std::fs::read_to_string(path)?;
    let reduced: ReducedOutput = serde_json::from_str(&content)?;

    let findings: Vec<(String, crate::parser::Finding)> = reduced
        .findings
        .into_iter()
        .map(|rf| {
            let priority = rf.priority.parse::<Priority>().unwrap_or_default();
            let finding = crate::parser::Finding {
                id: rf.id,
                finding_type: rf.finding_type,
                title: rf.title,
                priority,
                file: rf.file,
                line: rf.line,
                snippet: rf.snippet,
                description: rf.description,
                remediation: rf.remediation,
                acceptance_criteria: rf.acceptance_criteria,
                references: rf.references,
                model: rf.model,
            };
            // Use "reduced" as the reviewer_id for postprocessed findings
            ("reduced".to_string(), finding)
        })
        .collect();

    info!("Loaded {} reduced findings from {:?}", findings.len(), path);
    Ok(findings)
}
