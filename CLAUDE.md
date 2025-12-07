# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build and Development Commands

```bash
# Build
cargo build --release

# Run tests
cargo test
cargo test -- --nocapture  # with output

# Lint and format
cargo clippy -- -D warnings
cargo fmt --check
cargo fmt  # to fix

# Quick compile check
cargo check

# Install locally
cargo install --path .
```

## Running the Tool

```bash
# Initialize: analyze repo, generate config + prompts
cargo run -- init --dry-run
cargo run -- init --force

# Run reviewers
cargo run -- run --config examples/polyrev.yaml
cargo run -- run --config examples/polyrev.yaml --dry-run

# Create GitHub issues from findings
cargo run -- issue --config polyrev.yaml --repo owner/repo

# Print JSON schema
cargo run -- schema
```

## Architecture

Polyrev is a parallel code review orchestrator that runs multiple AI-powered reviewers concurrently using Claude Code CLI or Codex CLI.

### Module Structure

- **cli/** - Command handlers (`run`, `issue`, `init`, `schema` subcommands)
  - `init.rs` - Repo analysis (languages, frameworks) + AI-powered config/prompt generation
- **config/** - YAML config parsing and types, including `Reviewer`, `Scope`, and `Priority` definitions
- **discovery/** - File discovery: scope resolution, glob/ignore patterns, git diff filtering
- **provider/** - CLI provider abstraction (`claude.rs`, `codex.rs`) - spawns external CLIs and parses output
- **runner/** - Orchestration layer
  - `orchestrator.rs` - Semaphore-controlled concurrent reviewer execution
  - `executor.rs` - Single reviewer execution with chunking for large file sets
  - `retry.rs` - Retry logic for transient failures
- **parser/** - Extracts JSON findings from raw CLI output (`finding.rs`, `json.rs`, `markdown.rs`)
- **postprocess/** - Post-run aggregation: collects findings, invokes AI for deduplication/clustering
- **github/** - Issue creation and deduplication against existing issues
- **output/** - Report generation (markdown + JSON findings, summary reports)
- **state.rs** - Daily idempotency tracking (stores last run timestamps)

### Key Data Flow

1. Config loaded → scopes resolved to file lists → files chunked if exceeding `max_files`
2. Orchestrator spawns reviewer tasks up to `concurrency` limit
3. Each executor invokes provider CLI with prompt + file content
4. Parser extracts JSON findings array from CLI output
5. Reports written immediately as each reviewer completes (streaming)
6. Issue command reads findings JSON and creates GitHub issues via `gh` CLI

### Finding Schema

Reviewers output JSON arrays with this structure (see `src/parser/finding.rs`):
- Required: `id`, `title`, `file`, `description`, `remediation`
- Optional: `type`, `priority` (p0/p1/p2), `line`, `snippet`, `acceptance_criteria`, `references`
