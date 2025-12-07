use std::path::PathBuf;

pub fn default_version() -> u32 {
    1
}

pub fn default_target() -> PathBuf {
    PathBuf::from(".")
}

pub fn default_concurrency() -> usize {
    6
}

pub fn default_report_dir() -> PathBuf {
    PathBuf::from("reports")
}

pub fn default_timeout_sec() -> u64 {
    300
}

pub fn default_max_files() -> usize {
    50
}

pub fn default_launch_delay_ms() -> u64 {
    500
}

pub fn default_claude_binary() -> PathBuf {
    // Check common install location first
    if let Some(home) = std::env::var_os("HOME") {
        let local_path = PathBuf::from(home).join(".claude/local/claude");
        if local_path.exists() {
            return local_path;
        }
    }
    // Fall back to PATH lookup
    PathBuf::from("claude")
}

pub fn default_claude_tools() -> Vec<String> {
    vec!["Read".to_string(), "Grep".to_string(), "Glob".to_string()]
}

pub fn default_permission_mode() -> String {
    "acceptEdits".to_string()
}

pub fn default_codex_binary() -> PathBuf {
    PathBuf::from("codex")
}

pub fn default_codex_model() -> String {
    "gpt-4.1".to_string()
}

pub fn default_max_attempts() -> u32 {
    3
}

pub fn default_backoff_base_ms() -> u64 {
    1000
}

pub fn default_true() -> bool {
    true
}

pub fn default_postprocess_tool() -> String {
    "claude_cli".to_string()
}

pub fn default_postprocess_prompt() -> std::path::PathBuf {
    std::path::PathBuf::from("prompts/reduce.md")
}

pub fn default_postprocess_timeout() -> u64 {
    300 // 5 minutes
}

pub fn default_postprocess_min_findings() -> usize {
    2 // Only reduce if there are at least 2 findings
}

pub fn default_false() -> bool {
    false
}
