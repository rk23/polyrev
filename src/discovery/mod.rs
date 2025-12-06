mod diff;
pub mod files;
mod scope;

pub use diff::get_changed_files;
pub use files::chunk_files;
pub use scope::resolve_scope;

use crate::config::{Config, Reviewer};
use crate::error::DiscoveryError;
use std::collections::HashSet;
use std::path::PathBuf;

/// Discover all files for a reviewer based on its scopes
pub fn discover_files_for_reviewer(
    config: &Config,
    reviewer: &Reviewer,
    diff_base: Option<&str>,
) -> Result<Vec<PathBuf>, DiscoveryError> {
    let mut all_files = HashSet::new();

    // Get changed files if diff_base is specified
    let changed_files: Option<HashSet<PathBuf>> = if let Some(base) = diff_base {
        Some(
            get_changed_files(&config.target, base)?
                .into_iter()
                .collect(),
        )
    } else {
        None
    };

    // Collect files from all scopes
    for scope_name in &reviewer.scopes {
        if let Some(scope) = config.scopes.get(scope_name) {
            let scope_files = resolve_scope(&config.target, scope)?;

            for file in scope_files {
                // If we have a changed files filter, only include changed files
                if let Some(ref changed) = changed_files {
                    if changed.contains(&file) {
                        all_files.insert(file);
                    }
                } else {
                    all_files.insert(file);
                }
            }
        }
    }

    let mut files: Vec<_> = all_files.into_iter().collect();
    files.sort();
    Ok(files)
}
