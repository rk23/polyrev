//! Types for parallel planning perspectives and task DAG generation

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A planning perspective configuration (analogous to Reviewer)
#[derive(Debug, Clone, Default, Deserialize, Serialize, JsonSchema)]
pub struct Perspective {
    /// Unique identifier (e.g., "architecture", "security")
    pub id: String,

    /// Human-readable name
    pub name: String,

    /// What this perspective focuses on
    pub focus: String,

    /// Path to the prompt file
    pub prompt_file: PathBuf,

    /// Whether this perspective is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

/// Output from a single planning perspective
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PlanFragment {
    /// Which perspective produced this
    pub perspective: String,

    /// One-line summary of this perspective's view
    #[serde(default)]
    pub summary: String,

    /// Tasks suggested by this perspective
    #[serde(default)]
    pub tasks: Vec<FragmentTask>,

    /// Concerns identified by this perspective
    #[serde(default)]
    pub concerns: Vec<Concern>,

    /// Questions that need human input
    #[serde(default)]
    pub questions: Vec<Question>,
}

/// A task suggested by a planning perspective
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FragmentTask {
    /// Task title
    pub title: String,

    /// Why this task matters from this perspective
    #[serde(default)]
    pub rationale: String,

    /// Files to create/modify
    #[serde(default)]
    pub files: TaskFiles,

    /// Other task titles this depends on
    #[serde(default)]
    pub dependencies: Vec<String>,

    /// How to verify completion
    #[serde(default)]
    pub acceptance_criteria: Vec<AcceptanceCriterion>,

    /// Estimated complexity (optional hint)
    #[serde(default)]
    pub complexity: Option<String>,
}

/// Files associated with a task
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct TaskFiles {
    /// Primary files to create/modify
    #[serde(default)]
    pub target: Vec<PathBuf>,

    /// Reference files for context
    #[serde(default)]
    pub context: Vec<PathBuf>,
}

/// An acceptance criterion for a task
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AcceptanceCriterion {
    /// What needs to be true
    pub criterion: String,

    /// How to verify it
    #[serde(default)]
    pub verification: String,
}

/// A concern raised by a planning perspective
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Concern {
    /// Description of the concern
    pub description: String,

    /// How serious is this
    #[serde(default)]
    pub severity: Severity,

    /// Which tasks this impacts
    #[serde(default)]
    pub affects: Vec<String>,
}

/// Severity level for concerns
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Low,
    #[default]
    Medium,
    High,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Low => write!(f, "low"),
            Severity::Medium => write!(f, "medium"),
            Severity::High => write!(f, "high"),
        }
    }
}

/// A question that needs human input
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Question {
    /// The question to ask
    pub question: String,

    /// Possible answers if known
    #[serde(default)]
    pub options: Vec<String>,

    /// Suggested default if any
    #[serde(default)]
    pub default: Option<String>,

    /// Additional context
    #[serde(default)]
    pub context: Option<String>,
}

/// The unified task DAG after reduction
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UnifiedPlan {
    /// The merged task list with correct ordering
    pub tasks: Vec<UnifiedTask>,

    /// Questions requiring human input before execution
    #[serde(default)]
    pub questions: Vec<UnifiedQuestion>,

    /// Risks identified by multiple perspectives
    #[serde(default)]
    pub risks: Vec<Risk>,

    /// Nice-to-have tasks deferred for later
    #[serde(default)]
    pub deferred: Vec<DeferredTask>,

    /// Summary of the reduction
    #[serde(default)]
    pub summary: Option<String>,
}

/// A task in the unified plan
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UnifiedTask {
    /// Unique ID (e.g., "impl-oauth-001")
    pub id: String,

    /// Task title
    pub title: String,

    /// Detailed description (merged from perspectives)
    #[serde(default)]
    pub description: String,

    /// Files involved
    #[serde(default)]
    pub files: TaskFiles,

    /// Task IDs this depends on
    #[serde(default)]
    pub depends_on: Vec<String>,

    /// How to verify completion
    #[serde(default)]
    pub acceptance_criteria: Vec<AcceptanceCriterion>,

    /// Which perspectives contributed to this task
    #[serde(default)]
    pub perspectives: Vec<String>,

    /// Which workflow should execute this
    #[serde(default)]
    pub workflow: Option<String>,

    /// Task priority
    #[serde(default)]
    pub priority: TaskPriority,
}

/// Priority for tasks
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskPriority {
    #[default]
    Normal,
    High,
    Critical,
}

/// A question with context from multiple perspectives
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UnifiedQuestion {
    /// The question
    pub question: String,

    /// Why this matters
    #[serde(default)]
    pub context: String,

    /// Which perspectives raised this
    #[serde(default)]
    pub raised_by: Vec<String>,

    /// Possible options
    #[serde(default)]
    pub options: Vec<String>,

    /// Which task IDs are blocked by this
    #[serde(default)]
    pub blocks: Vec<String>,

    /// User's answer (filled after approval)
    #[serde(default)]
    pub answer: Option<String>,
}

/// A risk identified by multiple perspectives
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Risk {
    /// Description of the risk
    pub description: String,

    /// Which perspectives flagged this
    #[serde(default)]
    pub raised_by: Vec<String>,

    /// How serious
    #[serde(default)]
    pub severity: Severity,

    /// Suggested mitigation
    #[serde(default)]
    pub mitigation: Option<String>,
}

/// A task deferred for later
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DeferredTask {
    /// Task title
    pub title: String,

    /// Why it's deferred
    #[serde(default)]
    pub rationale: String,
}

/// Result of running all planning perspectives
#[derive(Debug)]
#[allow(dead_code)]
pub struct PlanningResult {
    /// Fragments from each perspective
    pub fragments: Vec<PerspectiveResult>,

    /// Total duration
    pub total_duration: std::time::Duration,
}

/// Result from a single perspective
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PerspectiveResult {
    /// Perspective ID
    pub perspective_id: String,

    /// Perspective name
    pub perspective_name: String,

    /// Execution status
    pub status: PerspectiveStatus,

    /// The plan fragment (if successful)
    pub fragment: Option<PlanFragment>,

    /// Execution duration
    pub duration: std::time::Duration,
}

/// Status of perspective execution
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum PerspectiveStatus {
    Completed,
    Failed { error: String },
    Skipped { reason: String },
}

impl std::fmt::Display for PerspectiveStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PerspectiveStatus::Completed => write!(f, "completed"),
            PerspectiveStatus::Skipped { reason } => write!(f, "skipped: {}", reason),
            PerspectiveStatus::Failed { error } => write!(f, "failed: {}", error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_plan_fragment() {
        let json = r#"{
            "perspective": "architecture",
            "summary": "Need modular OAuth setup",
            "tasks": [
                {
                    "title": "Add OAuth config schema",
                    "rationale": "Config-first enables flexibility",
                    "files": {
                        "target": ["src/config/oauth.rs"],
                        "context": ["src/config/mod.rs"]
                    },
                    "dependencies": [],
                    "acceptance_criteria": [
                        {"criterion": "OAuthConfig struct defined", "verification": "compiles"}
                    ]
                }
            ],
            "concerns": [
                {"description": "Async middleware needed", "severity": "medium", "affects": ["Add middleware"]}
            ],
            "questions": [
                {"question": "Redis or in-memory for state?", "options": ["redis", "memory"], "default": "memory"}
            ]
        }"#;

        let fragment: PlanFragment = serde_json::from_str(json).unwrap();
        assert_eq!(fragment.perspective, "architecture");
        assert_eq!(fragment.tasks.len(), 1);
        assert_eq!(fragment.concerns.len(), 1);
        assert_eq!(fragment.questions.len(), 1);
    }

    #[test]
    fn test_parse_unified_plan() {
        let json = r#"{
            "tasks": [
                {
                    "id": "impl-001",
                    "title": "Add OAuth config",
                    "description": "Setup configuration for OAuth providers",
                    "depends_on": [],
                    "perspectives": ["architecture", "security"],
                    "acceptance_criteria": []
                }
            ],
            "questions": [
                {
                    "question": "Which state backend?",
                    "raised_by": ["architecture", "security"],
                    "options": ["redis", "memory"],
                    "blocks": ["impl-002"]
                }
            ],
            "risks": [
                {
                    "description": "Callback URL validation critical",
                    "raised_by": ["security", "api"],
                    "severity": "high",
                    "mitigation": "Whitelist domains"
                }
            ],
            "summary": "12 tasks from 5 perspectives"
        }"#;

        let plan: UnifiedPlan = serde_json::from_str(json).unwrap();
        assert_eq!(plan.tasks.len(), 1);
        assert_eq!(plan.questions.len(), 1);
        assert_eq!(plan.risks.len(), 1);
    }
}
