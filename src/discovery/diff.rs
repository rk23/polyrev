use crate::error::DiscoveryError;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Get list of files changed since the given base ref
pub fn get_changed_files(target: &Path, base: &str) -> Result<Vec<PathBuf>, DiscoveryError> {
    let output = Command::new("git")
        .current_dir(target)
        .args(["diff", "--name-only", base])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(DiscoveryError::GitDiff(stderr.to_string()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let files: Vec<PathBuf> = stdout
        .lines()
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .collect();

    Ok(files)
}
