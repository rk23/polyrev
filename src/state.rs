use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

const STATE_DIR: &str = ".polyrev";
const STATE_FILE: &str = "state.json";

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct State {
    pub reviewers: HashMap<String, ReviewerState>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReviewerState {
    pub last_run: DateTime<Utc>,
    pub findings_count: usize,
}

impl State {
    /// Load state from the target directory
    pub fn load(target: &Path) -> Self {
        let state_path = Self::state_path(target);
        if state_path.exists() {
            match fs::read_to_string(&state_path) {
                Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
                Err(_) => Self::default(),
            }
        } else {
            Self::default()
        }
    }

    /// Save state to the target directory
    pub fn save(&self, target: &Path) -> std::io::Result<()> {
        let state_dir = target.join(STATE_DIR);
        fs::create_dir_all(&state_dir)?;

        let state_path = state_dir.join(STATE_FILE);
        let json = serde_json::to_string_pretty(self)?;
        fs::write(state_path, json)
    }

    /// Check if a reviewer has already run today
    pub fn ran_today(&self, reviewer_id: &str) -> bool {
        if let Some(reviewer_state) = self.reviewers.get(reviewer_id) {
            let now = Utc::now();
            let last_run = reviewer_state.last_run;

            // Check if last run was within the last 24 hours
            now.signed_duration_since(last_run) < Duration::hours(24)
        } else {
            false
        }
    }

    /// Record that a reviewer has run
    pub fn record_run(&mut self, reviewer_id: &str, findings_count: usize) {
        self.reviewers.insert(
            reviewer_id.to_string(),
            ReviewerState {
                last_run: Utc::now(),
                findings_count,
            },
        );
    }

    fn state_path(target: &Path) -> PathBuf {
        target.join(STATE_DIR).join(STATE_FILE)
    }
}
