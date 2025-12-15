mod defaults;
mod types;

pub use types::*;

use crate::error::ConfigError;
use defaults::*;
use std::collections::HashMap;
use std::path::Path;

impl Default for Config {
    fn default() -> Self {
        Self {
            version: default_version(),
            target: default_target(),
            concurrency: default_concurrency(),
            report_dir: default_report_dir(),
            dry_run: false,
            diff_base: None,
            github: GithubConfig::default(),
            providers: ProvidersConfig::default(),
            retry: RetryConfig::default(),
            postprocess: PostProcessConfig::default(),
            planning: None,
            timeout_sec: default_timeout_sec(),
            max_files: default_max_files(),
            launch_delay_ms: default_launch_delay_ms(),
            scopes: HashMap::new(),
            reviewers: Vec::new(),
        }
    }
}

impl Config {
    /// Load config from a YAML file
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path).map_err(|e| ConfigError::ReadFile {
            path: path.to_path_buf(),
            source: e,
        })?;

        let config: Config = serde_yaml::from_str(&content)?;
        Ok(config)
    }

    /// Validate the config
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Check that all reviewer scopes exist
        for reviewer in &self.reviewers {
            for scope_name in &reviewer.scopes {
                if !self.scopes.contains_key(scope_name) {
                    return Err(ConfigError::UnknownScope(scope_name.clone()));
                }
            }
        }

        // Check at least one reviewer is enabled
        let enabled_count = self.reviewers.iter().filter(|r| r.enabled).count();
        if enabled_count == 0 {
            return Err(ConfigError::NoReviewersEnabled);
        }

        Ok(())
    }
}
