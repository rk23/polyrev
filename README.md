# polyrev

Parallel code review orchestrator for Claude Code and Codex CLI.

Runs multiple specialized reviewers concurrently against a codebase, parses structured findings, deduplicates with AI, and creates GitHub issues that trigger automatic fixes.

## Installation

```bash
cargo install --path .
```

## Quick Start

```bash
# Initialize: analyze repo, generate config and prompts
polyrev init

# Run reviews + create issues (all-in-one)
polyrev run --create-issues

# Or run separately:
polyrev run                    # Run reviews + postprocess
polyrev issue                  # Create GitHub issues
```

## Full Automated Flow

With all features enabled, polyrev provides end-to-end automated code review:

```
polyrev run → reviewers → findings → postprocess → reduced.json
                                                        ↓
polyrev issue → GitHub issues → @claude → auto-fix PRs
```

**Setup for full automation:**

1. **Configure `polyrev.yaml`:**
```yaml
postprocess:
  enabled: true

github:
  repo: owner/repo
  auto_fix:
    enabled: true
    agent: claude
```

2. **Add GitHub Action to your repo** (`.github/workflows/claude.yml`):
```yaml
name: Claude Code
on:
  issues:
    types: [opened]
jobs:
  claude:
    if: contains(github.event.issue.body, '@claude')
    runs-on: ubuntu-latest
    permissions:
      contents: write
      pull-requests: write
      issues: write
    steps:
      - uses: actions/checkout@v4
      - uses: anthropics/claude-code-action@v1
        with:
          claude_code_oauth_token: ${{ secrets.CLAUDE_CODE_OAUTH_TOKEN }}
```

3. **Run the pipeline:**
```bash
polyrev run --config polyrev.yaml --create-issues
```

Issues are created with `@claude` in the body, triggering the GitHub Action to automatically create fix PRs.

## Usage

### Initializing a Repository

The `init` command analyzes your codebase and generates a tailored configuration:

```bash
# Analyze repo, generate polyrev.yaml + 3 review prompts
polyrev init

# Generate more reviewers (1-6)
polyrev init --reviewers 5

# Preview without writing files
polyrev init --dry-run

# Overwrite existing config/prompts
polyrev init --force

# Also create GitHub labels for issue tracking
polyrev init --labels --repo owner/repo

# Use Codex instead of Claude for generation
polyrev init --provider codex_cli
```

**What it does:**
1. Scans your repo (respects `.gitignore`)
2. Detects languages, frameworks, and directory structure
3. Invokes AI to generate tailored config and review prompts
4. Optionally creates GitHub labels (p0, p1, p2, polyrev, automated-review)

### Running Reviews

```bash
# Run all reviewers
polyrev run --config polyrev.yaml

# Run reviews + create GitHub issues (all-in-one)
polyrev run --config polyrev.yaml --create-issues

# Dry run (show execution plan without running)
polyrev run --config polyrev.yaml --dry-run

# Run specific reviewers only
polyrev run --config polyrev.yaml --reviewers security-python,api-contracts

# Run specific scopes only (for monorepos)
polyrev run --config polyrev.yaml --scopes backend

# Only review changed files since base branch
polyrev run --config polyrev.yaml --diff-base main

# Force re-run reviewers that already ran today
polyrev run --config polyrev.yaml --force

# CI mode: exit 1 if any p0 (critical) findings
polyrev run --config polyrev.yaml --fail-on-critical

# Verbose output
polyrev run --config polyrev.yaml --verbose
```

### Creating GitHub Issues

```bash
# Create issues from today's reports
# Uses reduced.json if postprocessing ran, otherwise raw findings
polyrev issue --config polyrev.yaml

# Preview issues without creating (dry run)
polyrev issue --config polyrev.yaml --dry-run

# Create issues from specific report directory
polyrev issue --report-dir reports/2024-01-15 --config polyrev.yaml

# Create issues from specific findings files
polyrev issue security-python.findings.json --config polyrev.yaml

# Skip deduplication (create even if similar issue exists)
polyrev issue --config polyrev.yaml --force
```

**With `auto_fix.enabled: true`**, each created issue includes:
```markdown
## Auto-Fix

@claude Please fix this issue following the remediation guidance above and create a pull request with your changes.
```

This triggers the Claude Code GitHub Action to automatically implement fixes.

### Other Commands

```bash
# Print JSON schema for config validation
polyrev schema > polyrev.schema.json

# Create GitHub labels only
polyrev init --labels --repo owner/repo
```

## Configuration

See `examples/polyrev.yaml` for a full example.

```yaml
version: 1
target: "."
concurrency: 6
report_dir: reports/
max_files: 50
timeout_sec: 300
launch_delay_ms: 500

# GitHub integration
github:
  repo: owner/repo
  labels: ["polyrev", "automated-review"]
  dedupe: true
  dedupe_action: skip  # skip, comment, or reopen

  # Auto-fix: trigger AI agent on new issues
  auto_fix:
    enabled: false
    agent: claude  # or codex
    prompt: "Please fix this issue following the remediation guidance above and create a pull request with your changes."

# Provider configuration
providers:
  claude_cli:
    model: claude-opus-4-5-20251101
    tools: ["Read", "Grep", "Glob"]
    permission_mode: acceptEdits
  codex_cli:
    model: gpt-5.1-codex-max

# Retry settings
retry:
  max_attempts: 3
  backoff_base_ms: 1000

# Postprocessing: deduplicate and cluster findings
postprocess:
  enabled: false
  tool: claude_cli
  prompt_file: prompts/reduce.md
  timeout_sec: 600
  min_findings: 2

# Scopes define file sets
scopes:
  backend:
    paths: [src/]
    include: ["**/*.py"]
    exclude: ["**/*_test.py"]

# Reviewers run against scopes
reviewers:
  - id: security-python
    name: Python Security Audit
    enabled: true
    provider: claude_cli
    scopes: [backend]
    prompt_file: prompts/security-python.md
    priority_default: p1
    max_files: 30
    timeout_sec: 600
```

## Providers

- `claude_cli` - Uses Claude Code CLI (`claude -p`)
- `codex_cli` - Uses Codex CLI (`codex exec`)

Both use your existing CLI subscription (not API keys).

## Writing Prompts

Each reviewer needs a prompt file that instructs the model what to look for. Prompts must instruct the model to output findings as a JSON array:

```json
[
  {
    "id": "SEC-001",
    "type": "sql-injection",
    "title": "SQL Injection in user query",
    "priority": "p0",
    "file": "src/db/users.py",
    "line": 42,
    "snippet": "query = f\"SELECT * FROM users WHERE id = {user_id}\"",
    "description": "User input is directly interpolated into SQL query...",
    "remediation": "Use parameterized queries",
    "acceptance_criteria": ["All SQL queries use parameterized statements"],
    "references": ["https://owasp.org/www-community/attacks/SQL_Injection"]
  }
]
```

### Field Reference

| Field | Required | Description |
|-------|----------|-------------|
| `id` | Yes | Unique identifier (e.g., `SEC-001`, `API-042`) |
| `type` | No | Category of finding |
| `title` | Yes | Short summary of the issue |
| `priority` | No | `p0` (critical), `p1` (high), or `p2` (medium) |
| `file` | Yes | Relative path to the file |
| `line` | No | Line number |
| `snippet` | No | Relevant code snippet |
| `description` | Yes | Detailed explanation |
| `remediation` | Yes | How to fix the issue |
| `acceptance_criteria` | No | Checklist items for the fix |
| `references` | No | Links to documentation, CVEs, etc. |

## Features

### Postprocessing (Deduplication)

When `postprocess.enabled: true`, polyrev runs an AI step after all reviewers complete to:
- Deduplicate similar findings across reviewers
- Cluster related issues
- Merge findings that point to the same root cause

Output is written to `reduced.json` in the report directory. The `issue` command automatically uses `reduced.json` when available.

### Auto-Fix Integration

When `github.auto_fix.enabled: true`, created issues include an `@claude` (or `@codex`) mention that triggers the Claude Code GitHub Action to automatically:
1. Analyze the issue
2. Implement the fix
3. Create a pull request

Requires the Claude Code GitHub Action in your repo with `CLAUDE_CODE_OAUTH_TOKEN` secret.

### Daily Idempotency

Each reviewer only runs once per day by default. Use `--force` to re-run.

### Intelligent Chunking

Large file sets are automatically split into chunks. For multi-chunk reviews, polyrev uses session resumption to maintain context.

### Issue Deduplication

The `issue` command checks for existing issues with the same fingerprint before creating new ones. Configurable via `dedupe_action`:
- `skip` - Don't create duplicate issues
- `comment` - Add comment to existing issue
- `reopen` - Reopen closed issues and comment

## Output

Reports are written to dated directories: `reports/YYYY-MM-DD/`

| File | Description |
|------|-------------|
| `{reviewer_id}.md` | Per-reviewer markdown report |
| `{reviewer_id}.findings.json` | Raw findings from reviewer |
| `summary.json` / `summary.md` | Aggregate summary |
| `reduced.json` | Deduplicated findings (when postprocess enabled) |

## License

MIT
