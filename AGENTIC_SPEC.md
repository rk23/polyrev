# Agentic: Unified AI Coding Orchestrator

## Vision

One CLI that handles the full agentic coding loop: plan → implement → review → ship.

```
agentic plan "Add OAuth support"     # AI generates task DAG from spec
agentic run                          # Orchestrator executes tasks
agentic review                       # Run code reviewers on changes
agentic issue                        # Create GitHub issues from findings
agentic tui                          # Real-time dashboard
```

Built on two foundations:
- **Tandem**: Task queue with DAG dependencies, quotas, resource locks
- **Polyrev**: Parallel perspectives with structured output → reduce/merge

The key insight: Polyrev's pattern isn't "code review" - it's **parallel perspectives → structured output → reduce**. This applies to planning, implementation review, and code review alike.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                              agentic CLI                                │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐       │
│  │  init   │  │  plan   │  │   run   │  │ review  │  │  issue  │       │
│  └────┬────┘  └────┬────┘  └────┬────┘  └────┬────┘  └────┬────┘       │
│       │            │            │            │            │             │
├───────┴────────────┴────────────┴────────────┴────────────┴─────────────┤
│                                                                         │
│                         Workflow Engine                                 │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐                  │
│  │   plan.*     │  │ implement.*  │  │   review.*   │                  │
│  │  (workflows) │  │  (workflows) │  │  (workflows) │                  │
│  └──────────────┘  └──────────────┘  └──────────────┘                  │
│                                                                         │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│                      Tandem Task Queue                                  │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │  SQLite: tasks, edges, quotas, domains, resources, workers      │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                                                         │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│                        Worker Pool                                      │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐                    │
│  │ claude  │  │ claude  │  │ claude  │  │ claude  │  (headless -p)     │
│  └─────────┘  └─────────┘  └─────────┘  └─────────┘                    │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## The Parallel Perspectives Model

The core pattern that unifies planning and review:

```
┌────────────────────────────────────────────────────────────────────────┐
│                     Parallel Perspectives Pattern                       │
├────────────────────────────────────────────────────────────────────────┤
│                                                                        │
│   Input (spec or code)                                                 │
│          │                                                             │
│          ▼                                                             │
│   ┌──────────────────────────────────────────────────────────┐        │
│   │              Parallel Perspective Workers                 │        │
│   │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐        │        │
│   │  │ persp.  │ │ persp.  │ │ persp.  │ │ persp.  │        │        │
│   │  │   A     │ │   B     │ │   C     │ │   D     │        │        │
│   │  └────┬────┘ └────┬────┘ └────┬────┘ └────┬────┘        │        │
│   └───────┼──────────┼──────────┼──────────┼────────────────┘        │
│           │          │          │          │                          │
│           ▼          ▼          ▼          ▼                          │
│   ┌─────────────────────────────────────────────────────────┐         │
│   │              Structured Fragments                        │         │
│   │  [fragment A] [fragment B] [fragment C] [fragment D]     │         │
│   └─────────────────────────┬───────────────────────────────┘         │
│                             │                                          │
│                             ▼                                          │
│                      ┌────────────┐                                    │
│                      │   Reduce   │                                    │
│                      └─────┬──────┘                                    │
│                            │                                           │
│                            ▼                                           │
│                    Unified Output                                      │
│                                                                        │
└────────────────────────────────────────────────────────────────────────┘
```

### Applied to Planning

```
Spec: "Add OAuth with Google and GitHub"
                    │
    ┌───────────────┼───────────────┬───────────────┬───────────────┐
    ▼               ▼               ▼               ▼               ▼
┌────────┐    ┌────────┐    ┌────────┐    ┌────────┐    ┌────────┐
│ arch   │    │ test   │    │security│    │  api   │    │ incre- │
│planner │    │planner │    │planner │    │planner │    │ mental │
└───┬────┘    └───┬────┘    └───┬────┘    └───┬────┘    └───┬────┘
    │             │             │             │             │
    ▼             ▼             ▼             ▼             ▼
[6 tasks]    [4 tasks]     [5 tasks]    [4 tasks]     [8 tasks]
[2 concerns] [1 concern]   [3 concerns] [0 concerns]  [1 concern]
    │             │             │             │             │
    └─────────────┴─────────────┴─────────────┴─────────────┘
                                │
                                ▼
                         ┌────────────┐
                         │plan.reduce │
                         └─────┬──────┘
                               │
                               ▼
                    ┌──────────────────────┐
                    │ 12 tasks (DAG)       │
                    │ 3 questions for human│
                    │ 2 shared risks       │
                    └──────────────────────┘
```

### Applied to Review

```
Changed files (from git diff)
                    │
    ┌───────────────┼───────────────┬───────────────┐
    ▼               ▼               ▼               ▼
┌────────┐    ┌────────┐    ┌────────┐    ┌────────┐
│security│    │contract│    │patterns│    │ perf   │
│reviewer│    │reviewer│    │reviewer│    │reviewer│
└───┬────┘    └───┬────┘    └───┬────┘    └───┬────┘
    │             │             │             │
    ▼             ▼             ▼             ▼
[findings]   [findings]    [findings]    [findings]
    │             │             │             │
    └─────────────┴─────────────┴─────────────┘
                        │
                        ▼
                 ┌────────────┐
                 │review.reduce│
                 └─────┬──────┘
                       │
                       ▼
              ┌────────────────────┐
              │ Deduplicated       │
              │ findings + issues  │
              └────────────────────┘
```

### Plan Fragment Schema

Each planning perspective outputs:

```yaml
# Output from a single planning perspective
perspective: string           # e.g., "architecture", "security", "testing"
summary: string               # One-line summary of this perspective's view

tasks:
  - title: string             # Task title
    rationale: string         # Why this task matters from this perspective
    files:
      target: [string]        # Files to create/modify
      context: [string]       # Files to reference
    dependencies: [string]    # Other task titles this depends on
    acceptance_criteria:
      - criterion: string
        verification: string

concerns:                     # Issues this perspective identified
  - description: string
    severity: low | medium | high
    affects: [string]         # Which tasks this impacts

questions:                    # Decisions needed from human
  - question: string
    options: [string]         # Possible answers if known
    default: string           # Suggested default if any
```

### Plan Reduce Prompt

The reducer synthesizes all perspectives:

```markdown
# Plan Reduce System Prompt

You are synthesizing planning perspectives into a unified task DAG.

## Input
You receive plan fragments from multiple perspectives:
- architecture: system design, module boundaries, data flow
- testing: test strategy, edge cases, coverage needs
- security: auth, validation, attack surface, secrets
- api: interface design, backwards compatibility, errors
- incremental: smallest shippable slices, dependency ordering

## Your Job

1. **Merge overlapping tasks**
   - Multiple perspectives may suggest similar tasks
   - Combine them, preserving acceptance criteria from all
   - Use the most conservative estimate for dependencies

2. **Resolve conflicts**
   - If perspectives disagree, prefer: security > correctness > simplicity
   - Note the conflict in task description

3. **Build correct DAG**
   - Order dependencies correctly
   - Identify tasks that can run in parallel
   - Flag circular dependencies as errors

4. **Surface human decisions**
   - Collect all questions from perspectives
   - Add any new questions from conflicts
   - Questions block execution until answered

5. **Identify shared risks**
   - Concerns raised by 2+ perspectives = high confidence risk
   - Include in output for human awareness

## Output Schema

```yaml
tasks:
  - id: string                # Unique ID (e.g., impl-oauth-001)
    title: string
    description: string       # Merged from all perspectives
    files:
      target: [string]
      context: [string]
    depends_on: [string]      # Task IDs
    acceptance_criteria: [...]
    perspectives: [string]    # Which perspectives contributed
    workflow: string          # e.g., implement.rust

questions_for_human:
  - question: string
    context: string           # Why this matters
    raised_by: [string]       # Which perspectives
    options: [string]
    blocks: [string]          # Which task IDs are blocked

risks:
  - description: string
    raised_by: [string]       # Which perspectives (2+ = high confidence)
    severity: low | medium | high
    mitigation: string        # Suggested approach

deferred:                     # Nice-to-have, not blocking
  - title: string
    rationale: string
```
```

### The Reducible Abstraction

Both planning and review use the same reduce pattern:

```rust
/// Trait for outputs that can be merged from parallel perspectives
trait Reducible: Sized {
    /// Merge two outputs, combining their content
    fn merge(&self, other: &Self) -> Self;

    /// Detect conflicts between outputs
    fn conflicts_with(&self, other: &Self) -> Vec<Conflict>;

    /// Deduplicate items (tasks or findings)
    fn deduplicate(&mut self);
}

/// Plan fragments can be reduced
impl Reducible for PlanFragment {
    fn merge(&self, other: &Self) -> Self {
        // Combine tasks, merge overlapping by title similarity
        // Collect all concerns and questions
    }

    fn conflicts_with(&self, other: &Self) -> Vec<Conflict> {
        // Tasks with same files but different approaches
        // Contradictory dependency ordering
    }

    fn deduplicate(&mut self) {
        // Merge tasks with >80% title similarity
        // Combine acceptance criteria
    }
}

/// Findings can be reduced
impl Reducible for FindingSet {
    fn merge(&self, other: &Self) -> Self {
        // Combine all findings
    }

    fn conflicts_with(&self, other: &Self) -> Vec<Conflict> {
        // Same location, contradictory assessments
    }

    fn deduplicate(&mut self) {
        // Fingerprint-based dedup (file + line + type + snippet)
    }
}

/// Generic reduce loop
fn reduce<T: Reducible>(fragments: Vec<T>, reducer_prompt: &str) -> ReducedOutput<T> {
    // 1. Pairwise conflict detection
    let conflicts = detect_conflicts(&fragments);

    // 2. AI-powered merge with conflict resolution
    let merged = invoke_reducer(fragments, conflicts, reducer_prompt);

    // 3. Deduplicate result
    merged.deduplicate();

    // 4. Extract human decisions needed
    let questions = extract_questions(&merged, &conflicts);

    ReducedOutput { result: merged, questions, conflicts }
}
```

---

## Unified Task Schema

One task type for everything: planning, implementation, and review.

```yaml
# Core fields (required)
id: string              # Unique identifier (e.g., "impl-auth-001", "review-sec-001")
title: string           # Human-readable title
type: enum              # plan | implement | review | test | manual
status: enum            # queued | ready | leased | done | failed | canceled

# Work definition
description: string     # What needs to be done (markdown)
acceptance_criteria:    # How to verify completion
  - criterion: string
    verification: string  # How to check (e.g., "run tests", "manual review")

# Scoping
files:                  # Files this task touches
  target: [string]      # Primary files to modify/review
  context: [string]     # Reference files for understanding

# Dependencies (DAG)
depends_on: [string]    # Task IDs that must complete first
blocks: [string]        # Task IDs waiting on this task

# Execution hints
workflow: string        # Which workflow executes this (e.g., "implement.rust", "review.security")
priority: enum          # p0 | p1 | p2 (p0 = critical)
resources: [string]     # Exclusive resources needed (e.g., "db-migration", "api-schema")
domains: [string]       # Rate-limited domains (e.g., "api:github", "api:stripe")
estimated_chunks: int   # Hint for parallelization

# Metadata
created_at: timestamp
created_by: string      # "human", "plan.feature", "review.security"
source: object          # Origin reference (issue URL, finding ID, etc.)

# Output (filled by worker)
result:
  status: enum          # success | failure | blocked
  summary: string
  files_modified: [string]
  criteria_results: [{criterion, passed, evidence}]
  follow_up_tasks: [Task]  # New tasks spawned by this work
  findings: [Finding]      # For review tasks
```

### Task Type Semantics

| Type | Created By | Executed By | Output |
|------|------------|-------------|--------|
| `plan` | Human or AI | AI planner | Child tasks (DAG) |
| `implement` | Plan task | AI worker | Code changes |
| `review` | Implement task or human | AI reviewer | Findings |
| `test` | Implement task | Test runner | Pass/fail |
| `manual` | Any | Human | Approval |

---

## Unified Configuration

Single config file: `agentic.yaml`

```yaml
version: 1

# Project metadata
project:
  name: myapp
  root: .

# Queue settings (Tandem)
queue:
  db_path: .agentic/queue.sqlite3
  default_quota: 6              # Max concurrent workers
  lease_ttl_seconds: 3600       # Worker timeout

# Worker settings
workers:
  provider: claude              # claude | codex
  binary: claude                # Path to CLI
  concurrency: 6                # Max parallel workers
  timeout_sec: 1800             # Per-task timeout

# Scopes (shared across workflows)
scopes:
  backend:
    paths: [src/]
    include: ["**/*.rs"]
    exclude: ["**/tests/**"]

  frontend:
    paths: [web/]
    include: ["**/*.ts", "**/*.tsx"]

# Planning (parallel perspectives → reduce)
planning:
  perspectives:
    - id: architecture
      prompt: prompts/plan-architecture.md
      focus: "system design, module boundaries, data flow, patterns"

    - id: testing
      prompt: prompts/plan-testing.md
      focus: "test strategy, edge cases, fixtures, coverage"

    - id: security
      prompt: prompts/plan-security.md
      focus: "auth, validation, secrets, attack surface"

    - id: api
      prompt: prompts/plan-api.md
      focus: "interface design, backwards compat, errors, docs"

    - id: incremental
      prompt: prompts/plan-incremental.md
      focus: "smallest shippable slices, parallel work, dependencies"

  reducer:
    prompt: prompts/plan-reduce.md
    output_schema: task_dag

  # Block execution until human answers questions
  require_human_approval: true
  on_unresolved_questions: block  # block | ask | proceed_with_defaults

# Workflows
workflows:

  # Implementation workflows
  implement.rust:
    prompt: prompts/implement-rust.md
    scopes: [backend]
    output_schema: worker_result
    auto_review: true           # Queue review task on completion

  implement.typescript:
    prompt: prompts/implement-ts.md
    scopes: [frontend]
    output_schema: worker_result
    auto_review: true

  # Review workflows (from polyrev)
  review.security:
    prompt: prompts/review-security.md
    scopes: [backend, frontend]
    priority_default: p1
    output_schema: findings

  review.contracts:
    prompt: prompts/review-contracts.md
    scopes: [backend]
    priority_default: p0
    output_schema: findings

  review.patterns:
    prompt: prompts/review-patterns.md
    scopes: [backend, frontend]
    priority_default: p2
    output_schema: findings

# GitHub integration
github:
  repo: owner/repo
  labels:
    p0: "priority: critical"
    p1: "priority: high"
    p2: "priority: medium"
  auto_issue: true              # Create issues from p0/p1 findings

# Orchestrator settings
orchestrator:
  mode: auto                    # auto | manual | batch
  conflict_detection: true      # Prevent parallel file conflicts
  auto_review: true             # Queue reviews after implementation
  finding_to_task: p0           # Auto-create tasks from findings at this priority
```

---

## CLI Commands

### `agentic init`

Analyze repo and generate config + prompts.

```bash
agentic init                    # Interactive setup
agentic init --dry-run          # Preview without writing
agentic init --force            # Overwrite existing
agentic init --workflows 5      # Generate N implementation workflows
```

**Process:**
1. Scan repo structure (languages, frameworks, directories)
2. Generate `agentic.yaml` with detected scopes
3. Generate workflow prompts tailored to codebase
4. Initialize SQLite queue database
5. Optionally create GitHub labels

### `agentic plan`

Generate task DAG from a spec using parallel planning perspectives.

```bash
agentic plan "Add OAuth support with Google and GitHub providers"
agentic plan --file spec.md                    # From file
agentic plan --issue 123                       # From GitHub issue
agentic plan --dry-run                         # Preview tasks without queueing
agentic plan --perspectives arch,security      # Use specific perspectives only
agentic plan --skip-reduce                     # Output raw fragments (debugging)
```

**Process:**
1. Load spec (argument, file, or GitHub issue)
2. Analyze codebase for context (languages, frameworks, patterns)
3. **Phase 1: Perspectives** - Spawn parallel planning workers:
   - `plan.architecture` - system design, modules, data flow
   - `plan.testing` - test strategy, edge cases, coverage
   - `plan.security` - auth, validation, attack surface
   - `plan.api` - interface design, backwards compatibility
   - `plan.incremental` - smallest slices, dependency ordering
4. Collect plan fragments from all perspectives
5. **Phase 2: Reduce** - Invoke reducer to synthesize:
   - Merge overlapping tasks
   - Resolve conflicts (security > correctness > simplicity)
   - Build correct dependency DAG
   - Surface questions for human
   - Identify shared risks
6. Present plan to human for approval
7. On approval, enqueue tasks into Tandem queue

**Output:**
```
$ agentic plan "Add OAuth with Google and GitHub"

Phase 1: Gathering perspectives...
  ├─ architecture   [running]
  ├─ testing        [running]
  ├─ security       [running]
  ├─ api            [running]
  └─ incremental    [running]

  ... (30 seconds, parallel) ...

  ├─ architecture   ✓ 6 tasks, 2 concerns
  ├─ testing        ✓ 4 tasks, 1 concern
  ├─ security       ✓ 5 tasks, 3 concerns
  ├─ api            ✓ 4 tasks, 0 concerns
  └─ incremental    ✓ 8 tasks, 1 concern

Phase 2: Reducing to unified plan...
  Merging 27 task suggestions...
  Resolving 2 conflicts...
  Building dependency graph...

═══════════════════════════════════════════════════════════════════════

Questions requiring your input:

  1. OAuth state storage
     Context: Architecture suggests in-memory, Security prefers Redis
     Options: [memory] [redis] [database]
     Raised by: architecture, security

  2. Token encryption key rotation
     Context: No existing pattern found in codebase
     Options: [manual] [automatic-30d] [automatic-90d]
     Raised by: security

Risks identified (flagged by 2+ perspectives):

  ⚠ Callback URL validation - open redirect risk
    Raised by: security, api
    Mitigation: Whitelist allowed redirect domains in config

  ⚠ Async middleware migration needed
    Raised by: architecture, incremental
    Mitigation: Add tokio dependency, refactor middleware stack

═══════════════════════════════════════════════════════════════════════

Draft plan: 12 tasks

  ┌─────────────────────────────────────────────────────────────────────┐
  │ impl-oauth-001: Add OAuth configuration schema                      │
  │   perspectives: architecture, security, api                         │
  │   files: src/config/oauth.rs                                        │
  │                                                                     │
  │ impl-oauth-002: Implement OAuthProvider trait                       │
  │   perspectives: architecture, api                                   │
  │   files: src/auth/provider.rs                                       │
  │   depends: 001                                                      │
  │                                                                     │
  │ impl-oauth-003: Add CSRF state parameter handling          [security]
  │   perspectives: security                                            │
  │   files: src/auth/oauth/csrf.rs                                     │
  │   depends: 002                                                      │
  │                                                                     │
  │ impl-oauth-004: Add Google OAuth provider                           │
  │   perspectives: architecture, testing, incremental                  │
  │   files: src/auth/providers/google.rs                               │
  │   depends: 002, 003                                                 │
  │                                                                     │
  │ ... (8 more tasks)                                                  │
  └─────────────────────────────────────────────────────────────────────┘

? Answer questions and approve plan? (y/n/edit)

> y
> Question 1: redis
> Question 2: automatic-90d

Enqueueing 12 tasks...
  Ready to execute: 1 task
  Blocked: 11 tasks

Run `agentic run` to start execution.
```

### `agentic run`

Execute tasks from the queue.

```bash
agentic run                     # Orchestrator mode (Claude decides batching)
agentic run --batch             # Headless batch mode (run all ready tasks)
agentic run --task impl-001     # Run specific task
agentic run --workers 4         # Limit concurrent workers
agentic run --dry-run           # Preview execution plan
```

**Orchestrator Mode (default):**
1. Start Claude as orchestrator with MCP access to queue
2. Orchestrator peeks ready tasks
3. Groups non-conflicting tasks into batches
4. Spawns workers for batch
5. Handles results (complete/fail/retry)
6. Queues follow-up tasks (reviews, tests)
7. Repeats until queue empty or human intervenes

**Batch Mode:**
1. Get all ready tasks
2. Simple conflict detection (file overlap)
3. Run non-conflicting tasks in parallel
4. No AI orchestrator overhead
5. Good for CI/CD

### `agentic review`

Run code reviewers (polyrev workflow).

```bash
agentic review                          # Review all scopes
agentic review --scope backend          # Review specific scope
agentic review --diff-base main         # Only changed files
agentic review --workflow review.security  # Specific reviewer
agentic review --enqueue                # Queue as tasks (vs direct execution)
```

**Process:**
1. Resolve files from scopes
2. For each review workflow:
   - If `--enqueue`: create review tasks in queue
   - Else: execute directly (polyrev style)
3. Collect findings
4. Deduplicate across reviewers
5. Optionally create GitHub issues
6. Optionally create follow-up tasks from findings

### `agentic issue`

Create GitHub issues from findings.

```bash
agentic issue                           # From latest review
agentic issue --findings findings.json  # From specific file
agentic issue --dry-run                 # Preview without creating
agentic issue --priority p0,p1          # Filter by priority
```

### `agentic tui`

Real-time dashboard (Tandem's TUI).

```bash
agentic tui                     # Full dashboard
agentic tui --watch             # Auto-refresh mode
```

**Features:**
- Task queue view (ready/leased/done/failed)
- Worker activity
- DAG visualization
- Finding summary
- Keyboard navigation

### `agentic queue`

Direct queue manipulation.

```bash
agentic queue list                      # List all tasks
agentic queue peek                      # Show ready tasks
agentic queue describe impl-001         # Task details + DAG
agentic queue cancel impl-001           # Cancel task
agentic queue retry impl-001            # Retry failed task
agentic queue gc --older-than 7d        # Garbage collect
```

---

## Workflow Execution Model

### How Workflows Run

```
┌──────────────────────────────────────────────────────────────┐
│                      Workflow Executor                       │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  1. Load workflow config (prompt, scopes, schema)            │
│  2. Resolve files from scopes                                │
│  3. Build context (files, dependencies, project info)        │
│  4. Render prompt with context                               │
│  5. Spawn worker (claude -p --output-format json)            │
│  6. Parse structured output                                  │
│  7. Validate against schema                                  │
│  8. Handle result:                                           │
│     - implement: queue review task if auto_review            │
│     - review: create findings, optionally queue fix tasks    │
│     - plan: enqueue child tasks                              │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

### Worker Input

Workers receive structured context:

```json
{
  "task": {
    "id": "impl-oauth-003",
    "title": "Add Google OAuth provider",
    "description": "Implement GoogleOAuthProvider...",
    "acceptance_criteria": [
      {"criterion": "Implements OAuthProvider trait", "verification": "compiles"},
      {"criterion": "Handles token refresh", "verification": "unit test"},
      {"criterion": "Returns user profile", "verification": "integration test"}
    ],
    "files": {
      "target": ["src/auth/providers/google.rs"],
      "context": ["src/auth/provider.rs", "src/auth/providers/mod.rs"]
    }
  },
  "context": {
    "project_root": "/path/to/project",
    "related_tasks": [
      {"id": "impl-oauth-002", "status": "done", "summary": "Added OAuthProvider trait"}
    ],
    "tech_stack": ["rust", "axum", "sqlx"],
    "conventions": "See CONVENTIONS.md"
  },
  "workflow": {
    "name": "implement.rust",
    "prompt_hash": "abc123"
  }
}
```

### Worker Output

Workers return structured results:

```json
{
  "status": "success",
  "summary": "Implemented Google OAuth provider with token refresh support",
  "files_modified": ["src/auth/providers/google.rs", "src/auth/providers/mod.rs"],
  "criteria_results": [
    {"criterion": "Implements OAuthProvider trait", "passed": true, "evidence": "Compiles successfully"},
    {"criterion": "Handles token refresh", "passed": true, "evidence": "Added refresh_token() method"},
    {"criterion": "Returns user profile", "passed": true, "evidence": "Returns GoogleUserProfile struct"}
  ],
  "tests": {
    "ran": true,
    "passed": true,
    "command": "cargo test auth::providers::google",
    "summary": "3 passed"
  },
  "follow_up_tasks": [],
  "notes": "Used reqwest for HTTP client to match existing patterns"
}
```

---

## The Orchestrator

The key to closing the loop: Claude as orchestrator.

### Orchestrator Capabilities

The orchestrator has MCP access to:

```
# Queue operations
queue.peek()              # View ready tasks
queue.describe(id)        # Task details + dependencies
queue.lease(id)           # Claim task for worker
queue.complete(id, result)
queue.fail(id, error, retry?)
queue.enqueue(task)       # Add new task
queue.cancel(id)

# Worker operations
worker.spawn(task)        # Start headless Claude
worker.status(id)         # Check worker progress
worker.kill(id)           # Terminate worker

# Project operations
project.files(scope)      # List files in scope
project.read(path)        # Read file content
project.conflicts(tasks)  # Predict file conflicts

# Review operations
review.findings()         # Get current findings
review.enqueue(scope)     # Queue review tasks
```

### Orchestrator Behavior

```markdown
# Orchestrator System Prompt

You are orchestrating an AI coding pipeline. Your job is to:

1. **Batch wisely**: Group non-conflicting tasks for parallel execution
2. **Respect dependencies**: Never run a task before its dependencies complete
3. **Handle failures**: Retry transient failures, flag persistent ones for human review
4. **Trigger reviews**: Queue review tasks when implementation completes
5. **Create follow-ups**: Turn critical findings into implementation tasks

## Decision Loop

1. Peek ready tasks
2. Check for file conflicts between ready tasks
3. Batch non-conflicting tasks
4. Spawn workers for batch
5. Wait for results
6. Handle each result:
   - Success: mark complete, queue review if auto_review
   - Failure: retry if transient, else mark failed
7. Check for critical findings, create fix tasks if needed
8. Repeat until queue empty

## When to Ask Human

- Task failed 3 times
- Conflicting findings from reviewers
- Architectural decision needed
- External dependency blocked
```

### Orchestrator Session Example

```
Orchestrator: Peeking ready tasks...
  - impl-oauth-001: Add OAuth configuration schema [ready]
  - impl-oauth-002: Implement OAuth provider trait [ready]

Orchestrator: Checking file conflicts...
  - impl-oauth-001 targets: src/config/oauth.rs (new file)
  - impl-oauth-002 targets: src/auth/provider.rs (new file)
  - No conflicts detected.

Orchestrator: Spawning workers for batch...
  - Worker claude-a1b2 → impl-oauth-001
  - Worker claude-c3d4 → impl-oauth-002

[2 minutes later]

Worker claude-a1b2: impl-oauth-001 complete
  - Created src/config/oauth.rs
  - All criteria passed

Worker claude-c3d4: impl-oauth-002 complete
  - Created src/auth/provider.rs
  - All criteria passed

Orchestrator: Both tasks complete. Checking for newly ready tasks...
  - impl-oauth-003: Add Google OAuth provider [now ready]
  - impl-oauth-004: Add GitHub OAuth provider [now ready]
  - impl-oauth-005: Add OAuth callback routes [now ready]

Orchestrator: Queueing review task for completed work...
  - review-impl-001: Review OAuth foundation [queued]

Orchestrator: Checking file conflicts for next batch...
  - impl-oauth-003 targets: src/auth/providers/google.rs
  - impl-oauth-004 targets: src/auth/providers/github.rs
  - impl-oauth-005 targets: src/routes/oauth.rs
  - No conflicts detected.

Orchestrator: Spawning 3 workers...
```

---

## Finding → Task Flow

Critical for closing the loop: findings create tasks.

### Configuration

```yaml
orchestrator:
  finding_to_task:
    p0: always              # Always create task for critical findings
    p1: ask                 # Ask human before creating task
    p2: never               # Just log, don't create task
```

### Flow

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Review    │────▶│   Finding   │────▶│    Task     │
│   Task      │     │   (p0)      │     │   (fix)     │
└─────────────┘     └─────────────┘     └─────────────┘
                           │
                           ▼
                    ┌─────────────┐
                    │   GitHub    │
                    │   Issue     │
                    └─────────────┘
```

### Finding → Task Transform

```yaml
# Finding
id: SEC-001
type: sql-injection
title: Raw SQL query in user lookup
file: src/db/users.rs
line: 42
priority: p0
description: |
  The query uses string interpolation...
remediation: |
  Use parameterized query with sqlx::query!()...
acceptance_criteria:
  - No string interpolation in SQL
  - Uses sqlx::query! or query_as!
  - Passes sqlx compile-time checks

# Generated Task
id: fix-SEC-001
type: implement
title: "Fix: Raw SQL query in user lookup"
description: |
  Finding SEC-001 identified a SQL injection vulnerability.

  **Location:** src/db/users.rs:42
  **Issue:** The query uses string interpolation...
  **Fix:** Use parameterized query with sqlx::query!()...
acceptance_criteria:
  - No string interpolation in SQL
  - Uses sqlx::query! or query_as!
  - Passes sqlx compile-time checks
  - Original tests still pass
files:
  target: [src/db/users.rs]
source:
  type: finding
  finding_id: SEC-001
  review_task_id: review-impl-001
workflow: implement.rust
priority: p0
```

---

## Directory Structure

```
project/
├── agentic.yaml              # Unified config
├── .agentic/
│   ├── queue.sqlite3         # Tandem task queue
│   ├── state.json            # Run state (idempotency)
│   ├── cache/                # File hashes, finding fingerprints
│   └── plans/                # Saved plan fragments for debugging
│       └── 2024-01-15-oauth/
│           ├── architecture.fragment.json
│           ├── security.fragment.json
│           ├── testing.fragment.json
│           ├── api.fragment.json
│           ├── incremental.fragment.json
│           └── reduced.plan.json
├── prompts/
│   ├── plan/                 # Planning perspective prompts
│   │   ├── architecture.md   # System design perspective
│   │   ├── testing.md        # Test strategy perspective
│   │   ├── security.md       # Security perspective
│   │   ├── api.md            # API design perspective
│   │   ├── incremental.md    # Incremental delivery perspective
│   │   └── reduce.md         # Reducer prompt
│   ├── implement/            # Implementation workflows
│   │   ├── rust.md
│   │   └── typescript.md
│   └── review/               # Review workflows
│       ├── security.md
│       ├── contracts.md
│       ├── patterns.md
│       └── reduce.md         # Finding reducer
└── reports/
    └── 2024-01-15/
        ├── summary.md
        ├── summary.json
        ├── review-security.findings.json
        └── review-contracts.findings.json
```

---

## Implementation Phases

Prioritized based on learnings: planning quality and human-in-the-loop ergonomics matter most.

### Phase 1: Parallel Planning (The Core Value)
- Implement parallel perspectives pattern for planning
- Plan fragment schema + reducer
- Questions/risks surfacing
- Human approval flow before enqueueing
- This is the "spec → tasks" magic that makes everything else work

### Phase 2: Unified CLI Shell
- Single `agentic` binary
- Shared config parsing (`agentic.yaml`)
- Delegates to tandem queue + polyrev workflows
- `agentic plan`, `agentic run`, `agentic review` commands

### Phase 3: Tandem as Library
- Extract tandem's queue as a Rust library crate
- Polyrev imports it (no subprocess, no separate SQLite)
- Shared task schema across both
- Single `.agentic/queue.sqlite3`

### Phase 4: Visibility & Escape Hatches
- Streaming worker progress (tool calls, file edits)
- Cost tracking per task (token counts)
- "Pause and ask" mechanism for workers
- Worker output in TUI (not just tool names)

### Phase 5: Orchestrator Mode
- Claude-as-orchestrator with MCP access to queue
- Conflict detection (file-level)
- Auto-review queueing
- Human intervention points

### Phase 6: Finding → Task Loop
- Finding to task conversion
- Configurable automation levels (`p0: always`, `p1: ask`)
- Closes the review → fix loop

### Phase 7: Polish & Adoption
- TUI enhancements (DAG view, plan browser, finding browser)
- GitHub PR integration (comment on PRs, not just issues)
- CI/CD batch mode
- Pre-built perspective prompts for common stacks
- Documentation + examples

---

## Open Questions

1. **Orchestrator trust level**: How much autonomy? Always ask before spawning workers? Auto-approve up to N tasks?

2. **Conflict detection granularity**: File-level? Function-level? Let workers figure it out?

3. **Review timing**: After every task? After every batch? On-demand only?

4. **Finding deduplication**: Across runs? Across reviews? Use embeddings for semantic dedup?

5. **Cost tracking**: Track token usage per task? Budget limits?

6. **Multi-repo**: Support orchestrating across multiple repos?

---

## Success Metrics

The loop is closed when:

1. `agentic plan "Add feature X"` generates a valid task DAG
2. `agentic run` executes all tasks with parallelism
3. Reviews automatically run on completed work
4. Critical findings become fix tasks
5. Human only intervenes for decisions, not mechanics
6. A non-trivial feature ships with minimal human coding

---

## Appendix: Example Session

Full end-to-end session showing plan → execute → review → ship.

```bash
$ agentic plan "Add rate limiting to API endpoints"

Analyzing codebase...
  - Language: Rust
  - Framework: axum
  - Existing middleware: src/middleware/
  - Config pattern: src/config/

Phase 1: Gathering perspectives...
  ├─ architecture   [running]
  ├─ testing        [running]
  ├─ security       [running]
  ├─ api            [running]
  └─ incremental    [running]

  ├─ architecture   ✓ 4 tasks, 1 concern
  ├─ testing        ✓ 3 tasks, 0 concerns
  ├─ security       ✓ 3 tasks, 2 concerns
  ├─ api            ✓ 2 tasks, 0 concerns
  └─ incremental    ✓ 5 tasks, 0 concerns

Phase 2: Reducing to unified plan...
  Merging 17 task suggestions...
  Resolving 1 conflict...
  Building dependency graph...

═══════════════════════════════════════════════════════════════════════

Questions requiring your input:

  1. Rate limit storage backend
     Context: Architecture suggests in-memory HashMap, Security prefers Redis
              for distributed deployments
     Options: [memory] [redis]
     Raised by: architecture, security

Risks identified (flagged by 2+ perspectives):

  ⚠ Rate limit bypass via header spoofing
    Raised by: security, api
    Mitigation: Use X-Forwarded-For only behind trusted proxy, validate

═══════════════════════════════════════════════════════════════════════

Draft plan: 7 tasks

  impl-rate-001: Add rate limit config schema
    perspectives: architecture, security, api
    files: src/config/rate_limit.rs

  impl-rate-002: Implement token bucket rate limiter
    perspectives: architecture, testing
    files: src/rate_limit/mod.rs, src/rate_limit/bucket.rs
    depends: 001

  impl-rate-003: Add rate limit middleware
    perspectives: architecture, api, incremental
    files: src/middleware/rate_limit.rs
    depends: 002

  impl-rate-004: Add rate limit response headers
    perspectives: api
    files: src/middleware/rate_limit.rs
    depends: 003

  impl-rate-005: Add IP extraction with proxy support      [security]
    perspectives: security
    files: src/middleware/client_ip.rs
    depends: 003

  impl-rate-006: Add rate limit integration tests
    perspectives: testing, incremental
    files: tests/rate_limit_test.rs
    depends: 004, 005

  impl-rate-007: Add rate limit documentation
    perspectives: api, incremental
    files: docs/rate-limiting.md
    depends: 006

? Answer questions and approve plan? (y/n/edit)
> y
> Question 1: memory  (we'll add redis later if needed)

Enqueueing 7 tasks...
  Ready: 1 task (impl-rate-001)
  Blocked: 6 tasks

$ agentic run

Orchestrator starting...

┌─ Queue Status ──────────────────────────────────────────────────────┐
│ Ready: 1    Leased: 0    Done: 0    Failed: 0                       │
└─────────────────────────────────────────────────────────────────────┘

[Batch 1]
  Spawning: impl-rate-001 (Add rate limit config schema)

  ┌─ Worker claude-x1y2 ────────────────────────────────────────────┐
  │ Task: impl-rate-001                                             │
  │ Status: running (0:45)                                          │
  │ Tokens: 12,340 in / 3,210 out ($0.24)                           │
  │                                                                 │
  │ Recent:                                                         │
  │   Read src/config/mod.rs                                        │
  │   Read src/config/server.rs (pattern reference)                 │
  │   Write src/config/rate_limit.rs ████████░░ 80%                 │
  └─────────────────────────────────────────────────────────────────┘

  impl-rate-001 complete (1m 12s, $0.31)
    Created: src/config/rate_limit.rs
    Modified: src/config/mod.rs
    Criteria: 3/3 passed

[Batch 2]
  Task impl-rate-002 now ready (dependency satisfied)
  Spawning: impl-rate-002 (Implement token bucket rate limiter)

  ... (2m 34s) ...

  impl-rate-002 complete ($0.45)
    Created: src/rate_limit/mod.rs, src/rate_limit/bucket.rs
    Tests: 4 passed

[Batch 3]
  Task impl-rate-003 now ready
  Spawning: impl-rate-003 (Add rate limit middleware)

  Queueing auto-review: review-batch-001 (config + rate_limit modules)

  ... (3m 15s) ...

  impl-rate-003 complete ($0.52)

[Batch 4 - parallel]
  Tasks ready: impl-rate-004, impl-rate-005
  No file conflicts detected

  Spawning: impl-rate-004 (Add rate limit response headers)
  Spawning: impl-rate-005 (Add IP extraction with proxy support)
  Spawning: review-batch-001 (Review rate limit foundation)

  ┌─ Workers ─────────────────────────────────────────────────────────┐
  │ claude-x1y2  impl-rate-004  [running 1:23]  $0.18                 │
  │ claude-a1b2  impl-rate-005  [running 1:45]  $0.22                 │
  │ claude-c3d4  review-batch-001  [running 0:34]  $0.15              │
  └───────────────────────────────────────────────────────────────────┘

  impl-rate-004 complete ($0.28)
  impl-rate-005 complete ($0.35)
  review-batch-001 complete
    Findings: 0 p0, 0 p1, 1 p2 (naming suggestion)

[Batch 5]
  Task impl-rate-006 now ready
  Spawning: impl-rate-006 (Add rate limit integration tests)

  ... (2m 48s) ...

  impl-rate-006 complete ($0.41)
    Tests: 8 passed (including new integration tests)

[Batch 6]
  Task impl-rate-007 now ready
  Spawning: impl-rate-007 (Add rate limit documentation)

  ... (1m 05s) ...

  impl-rate-007 complete ($0.19)

═══════════════════════════════════════════════════════════════════════

All tasks complete.

Summary:
  Tasks: 7 implementation, 1 review
  Duration: 14 minutes
  Cost: $2.71 (estimated)
  Files created: 6
  Files modified: 2
  Tests: 12 passed
  Findings: 1 p2 (style suggestion, not blocking)

$ agentic review --diff-base main

Running final review on all changes...

Phase 1: Running reviewers...
  ├─ security       [running]
  ├─ contracts      [running]
  ├─ patterns       [running]
  └─ performance    [running]

  ├─ security       ✓ 0 findings
  ├─ contracts      ✓ 0 findings
  ├─ patterns       ✓ 1 finding (p2)
  └─ performance    ✓ 0 findings

Phase 2: Reducing findings...
  1 total finding (deduplicated)

═══════════════════════════════════════════════════════════════════════

Findings:

  PAT-001 [p2] Consider extracting rate limit key generation
    File: src/middleware/rate_limit.rs:45
    The client identification logic could be a separate function
    for reuse in other middleware.

═══════════════════════════════════════════════════════════════════════

No critical findings. Ready to ship.

$ git add -A && git commit -m "Add rate limiting to API endpoints

- Token bucket algorithm with configurable limits
- Per-client rate limiting via IP (proxy-aware)
- Standard rate limit headers (X-RateLimit-*)
- Integration tests covering edge cases

Planned and implemented via agentic (7 tasks, 5 perspectives)"
```
