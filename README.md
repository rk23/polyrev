# polyrev

Parallel code review orchestrator for Claude Code and Codex CLI.

Runs multiple specialized reviewers concurrently against a codebase, parses structured findings, and creates GitHub issues with priority tags.

## Installation

```bash
cargo install --path .
```

## Quick Start

```bash
# Initialize: analyze repo, generate config and prompts
polyrev init

# Preview what will run
polyrev run --dry-run

# Run all reviewers
polyrev run
```

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

**Example output:**
```
Analyzing repository...

  Languages:
    Python (65%)
    TypeScript (30%)

  Frameworks:
    FastAPI
    React
    Prisma

  Structure:
    backend/ → Python (120 files)
    frontend/ → TypeScript (85 files)

Generating config via claude_cli...

Writing files:
  ✓ polyrev.yaml
  ✓ prompts/python-security.md
  ✓ prompts/api-contracts.md
  ✓ prompts/react-security.md
```

### Running Reviews

```bash
# Run all reviewers
polyrev run --config polyrev.yaml

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
```

### Creating GitHub Issues

```bash
# Create GitHub labels (if not done during init)
polyrev init --labels --repo owner/repo

# Create issues from today's reports (scans reports/<date>/)
polyrev issue --config polyrev.yaml --repo owner/repo

# Create issues from specific report directory
polyrev issue --report-dir reports/2024-01-15 --repo owner/repo

# Create issues from specific findings files
polyrev issue security-python.findings.json api-contracts.findings.json --repo owner/repo

# Preview issues without creating (dry run)
polyrev issue --report-dir reports/2024-01-15 --repo owner/repo --dry-run

# Skip deduplication (create even if similar issue exists)
polyrev issue --report-dir reports/ --repo owner/repo --no-dedup
```

### Other Commands

```bash
# Print JSON schema for config validation
polyrev schema > polyrev.schema.json
```

## Configuration

See `examples/polyrev.yaml` for a full example.

```yaml
version: 1
target: "."
concurrency: 6
report_dir: reports/
max_files: 50          # Files per chunk (for large repos)
timeout_sec: 300       # Default timeout per reviewer
launch_delay_ms: 500   # Delay between reviewer launches

github:
  repo: owner/repo     # Default repo for issue creation

scopes:
  backend:
    paths: [src/]
    include: ["**/*.py"]
    exclude: ["**/*_test.py"]

reviewers:
  - id: security-python
    name: Python Security Audit
    enabled: true
    provider: claude_cli
    scopes: [backend]
    prompt_file: prompts/security-python.md
    priority_default: p1
    max_files: 30      # Override default for this reviewer
    timeout_sec: 600   # Override timeout for this reviewer
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
    "id": "REVIEWER-001",
    "type": "sql-injection",
    "title": "SQL Injection in user query",
    "priority": "p0",
    "file": "src/db/users.py",
    "line": 42,
    "snippet": "query = f\"SELECT * FROM users WHERE id = {user_id}\"",
    "description": "User input is directly interpolated into SQL query without sanitization, allowing attackers to execute arbitrary SQL.",
    "remediation": "Use parameterized queries: `cursor.execute(\"SELECT * FROM users WHERE id = ?\", (user_id,))`",
    "acceptance_criteria": [
      "All SQL queries use parameterized statements",
      "Input validation added for user_id"
    ],
    "references": [
      "https://owasp.org/www-community/attacks/SQL_Injection"
    ]
  }
]
```

### Field Reference

| Field | Required | Description |
|-------|----------|-------------|
| `id` | Yes | Unique identifier (e.g., `SEC-001`, `API-042`) |
| `type` | No | Category of finding (e.g., `sql-injection`, `race-condition`) |
| `title` | Yes | Short summary of the issue |
| `priority` | No | `p0` (critical), `p1` (high), or `p2` (medium). Defaults to reviewer's `priority_default` |
| `file` | Yes | Relative path to the file |
| `line` | No | Line number (0 or omit if not applicable) |
| `snippet` | No | Relevant code snippet |
| `description` | Yes | Detailed explanation of the issue |
| `remediation` | Yes | How to fix the issue |
| `acceptance_criteria` | No | Checklist items for the fix |
| `references` | No | Links to documentation, CVEs, etc. |

See `prompts/` for example prompt files.

## Features

### Daily Idempotency
Each reviewer only runs once per day by default. Use `--force` to re-run.

### Intelligent Chunking
Large file sets are automatically split into chunks. For multi-chunk reviews, polyrev uses session resumption to maintain context across chunks - the model accumulates findings and only outputs on the final chunk.

### Streaming Reports
Reports are written immediately as each reviewer completes, so partial results are preserved if the run is interrupted.

### Postprocess Hook (Reducer-ready)
Optional postprocess step aggregates findings (`reduced.json`) so you can tag/cluster or feed downstream tools. Configure under `postprocess` in `polyrev.yaml` and set `enabled: true`.

## Output

Reports are written to dated directories: `reports/YYYY-MM-DD/`
- `{reviewer_id}.md` - Per-reviewer markdown report with findings
- `{reviewer_id}.findings.json` - Structured findings for issue creation
- `summary.json` / `summary.md` - Aggregate per run
- `reduced.json` - Aggregated findings when postprocess is enabled

## License

MIT
