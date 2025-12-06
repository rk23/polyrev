use crate::config::Scope;
use crate::error::DiscoveryError;
use globset::{Glob, GlobSetBuilder};
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

/// Resolve a scope to a list of files
pub fn resolve_scope(target: &Path, scope: &Scope) -> Result<Vec<PathBuf>, DiscoveryError> {
    // Build include globset
    let mut include_builder = GlobSetBuilder::new();
    for pattern in &scope.include {
        let glob = Glob::new(pattern).map_err(|e| DiscoveryError::GlobPattern {
            pattern: pattern.clone(),
            source: e,
        })?;
        include_builder.add(glob);
    }
    let include_set = include_builder
        .build()
        .map_err(|e| DiscoveryError::GlobPattern {
            pattern: "include set".to_string(),
            source: e,
        })?;

    // Build exclude globset
    let mut exclude_builder = GlobSetBuilder::new();
    for pattern in &scope.exclude {
        let glob = Glob::new(pattern).map_err(|e| DiscoveryError::GlobPattern {
            pattern: pattern.clone(),
            source: e,
        })?;
        exclude_builder.add(glob);
    }
    let exclude_set = exclude_builder
        .build()
        .map_err(|e| DiscoveryError::GlobPattern {
            pattern: "exclude set".to_string(),
            source: e,
        })?;

    let mut files = Vec::new();

    // Walk each path in the scope
    for scope_path in &scope.paths {
        let full_path = target.join(scope_path);
        if !full_path.exists() {
            continue;
        }

        // Use ignore crate to respect .gitignore
        let walker = WalkBuilder::new(&full_path)
            .hidden(true) // skip hidden files
            .git_ignore(true) // respect .gitignore
            .git_global(true) // respect global gitignore
            .git_exclude(true) // respect .git/info/exclude
            .build();

        for entry in walker {
            let entry = entry?;
            let path = entry.path();

            // Skip directories
            if path.is_dir() {
                continue;
            }

            // Get path relative to target for pattern matching
            let rel_path = path.strip_prefix(target).unwrap_or(path);

            // Check include patterns (if any specified)
            if !scope.include.is_empty() && !include_set.is_match(rel_path) {
                continue;
            }

            // Check exclude patterns
            if exclude_set.is_match(rel_path) {
                continue;
            }

            files.push(rel_path.to_path_buf());
        }
    }

    files.sort();
    Ok(files)
}
