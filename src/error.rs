use std::path::PathBuf;
use thiserror::Error;

#[allow(dead_code)]
#[derive(Error, Debug)]
pub enum PolyrevError {
    #[error("Config error: {0}")]
    Config(#[from] ConfigError),

    #[error("Discovery error: {0}")]
    Discovery(#[from] DiscoveryError),

    #[error("Provider error: {0}")]
    Provider(#[from] ProviderError),

    #[error("Runner error: {0}")]
    Runner(#[from] RunnerError),

    #[error("Parser error: {0}")]
    Parser(#[from] ParserError),

    #[error("Output error: {0}")]
    Output(#[from] OutputError),

    #[error("GitHub error: {0}")]
    GitHub(#[from] GitHubError),

    #[error("Postprocess error: {0}")]
    Postprocess(#[from] PostprocessError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to read config file '{path}': {source}")]
    ReadFile {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("Failed to parse config: {0}")]
    Parse(#[from] serde_yaml::Error),

    #[error("Unknown scope '{0}' referenced by reviewer")]
    UnknownScope(String),

    #[error("No reviewers enabled")]
    NoReviewersEnabled,
}

#[derive(Error, Debug)]
pub enum DiscoveryError {
    #[error("Failed to build glob pattern '{pattern}': {source}")]
    GlobPattern {
        pattern: String,
        #[source]
        source: globset::Error,
    },

    #[error("Failed to walk directory: {0}")]
    Walk(#[from] ignore::Error),

    #[error("Git diff failed: {0}")]
    GitDiff(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Error, Debug)]
pub enum ProviderError {
    #[error("Execution timed out after {0:?}")]
    Timeout(std::time::Duration),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Process failed with exit code {code}: {stderr}")]
    NonZeroExit { code: i32, stderr: String },
}

#[derive(Error, Debug)]
pub enum RunnerError {
    #[error("No reviewers matched filters")]
    NoReviewersMatched,

    #[error("Failed to acquire semaphore: {0}")]
    Semaphore(#[from] tokio::sync::AcquireError),

    #[error("Provider error: {0}")]
    Provider(#[from] ProviderError),

    #[error("Discovery error: {0}")]
    Discovery(#[from] DiscoveryError),
}

#[allow(dead_code)]
#[derive(Error, Debug)]
pub enum ParserError {
    #[error("Failed to parse JSON: {0}")]
    Json(#[from] serde_json::Error),

    #[error("No findings could be parsed from output")]
    NoFindings,

    #[error("Invalid finding format: {0}")]
    InvalidFormat(String),
}

#[derive(Error, Debug)]
pub enum OutputError {
    #[error("Failed to create output directory: {0}")]
    CreateDir(std::io::Error),

    #[error("Failed to write report: {0}")]
    WriteReport(std::io::Error),

    #[error("Serialization error: {0}")]
    Serialize(#[from] serde_json::Error),
}

#[derive(Error, Debug)]
pub enum GitHubError {
    #[error("gh CLI failed: {0}")]
    GhCli(String),

    #[error("Failed to parse gh output: {0}")]
    ParseOutput(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Issue creation failed: {0}")]
    CreateFailed(String),
}

#[derive(Error, Debug)]
pub enum PostprocessError {
    #[error("CLI execution failed: {0}")]
    CliExecution(String),

    #[error("Failed to parse reduced output: {0}")]
    ParseOutput(String),

    #[error("Execution timed out after {0:?}")]
    Timeout(std::time::Duration),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialize(#[from] serde_json::Error),
}
