//! Planning module: parallel perspectives for task DAG generation
//!
//! This module implements the "parallel perspectives → reduce" pattern for planning:
//! 1. Run multiple planning perspectives in parallel (architecture, security, testing, etc.)
//! 2. Each perspective outputs a PlanFragment with suggested tasks, concerns, questions
//! 3. Reduce/merge fragments into a UnifiedPlan with proper task DAG
//!
//! The pattern mirrors the review workflow but for planning instead of code review.

pub mod orchestrator;
pub mod parser;
pub mod reducer;
pub mod types;

pub use orchestrator::{select_perspectives, PlanOptions, PlanOrchestrator};
pub use reducer::{reduce_plan, revise_plan, write_fragments, write_plan};
pub use types::{
    PerspectiveResult, PerspectiveStatus, Perspective, Risk, Severity, UnifiedPlan,
    UnifiedQuestion, UnifiedTask,
};
