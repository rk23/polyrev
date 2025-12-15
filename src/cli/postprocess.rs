use crate::cli::PostprocessArgs;
use crate::config::Config;
use crate::postprocess::run_postprocess;
use chrono::Local;
use tracing::{error, info};

pub async fn execute(args: PostprocessArgs) -> anyhow::Result<()> {
    // Load config
    let config = Config::load(&args.config)?;

    // Determine report directory
    let report_dir = if let Some(dir) = args.report_dir {
        dir
    } else {
        // Default to today's report directory
        let date_str = Local::now().format("%Y-%m-%d").to_string();
        config.report_dir.join(&date_str)
    };

    if !report_dir.exists() {
        anyhow::bail!("Report directory not found: {:?}", report_dir);
    }

    info!("Running postprocess on {:?}", report_dir);

    // Run postprocess
    match run_postprocess(&config, &report_dir).await {
        Ok(Some(result)) => {
            info!(
                "Postprocess complete: {} -> {} findings",
                result.original_count, result.reduced_count
            );
            if !result.clusters.is_empty() {
                info!("Identified {} clusters", result.clusters.len());
            }
            if let Some(summary) = &result.summary {
                println!("\n{}", summary);
            }
        }
        Ok(None) => {
            info!("Postprocess skipped (disabled in config or no findings)");
        }
        Err(e) => {
            error!("Postprocess failed: {}", e);
            anyhow::bail!("Postprocess failed: {}", e);
        }
    }

    Ok(())
}
