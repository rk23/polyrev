# Repository Guidelines

## Project Structure & Module Organization
- Core library: `src/` with CLI entry in `src/main.rs` and submodules: `cli/` (commands), `config/` (YAML schema/defaults), `discovery/` (scopes, diffs, chunking), `provider/` (Claude & Codex runners), `runner/` (orchestration/retries), `parser/` (JSON/Markdown findings), `output/` (reports/summary), `github/` (dedupe + issue creation), `state.rs` (per-reviewer run cache).
- Prompts live in `prompts/`; sample config in `examples/polyrev.yaml`; generated reports land in `reports/` (git-ignored).
- Integration tests belong in `tests/`; unit tests sit alongside modules.

## Build, Test, and Development Commands
- `just run --config polyrev.yaml` or `cargo run -- run --config polyrev.yaml` — execute reviewers.
- `just dry-run` — print plan without calling providers.
- `just issue --report-dir reports` — file GitHub issues from prior run.
- `cargo check` — fast type/compile check.
- `cargo fmt --check` and `cargo clippy -- -D warnings` — formatting and lint.
- `cargo test` or `just test-verbose` — run unit/integration tests.
- `cargo build --release` or `just build` — release binary; `just install` installs locally.

## Coding Style & Naming Conventions
- Rust edition 2021; rely on `cargo fmt` (rustfmt defaults) and `clippy` with warnings-as-errors.
- Modules/files use `snake_case`; enums/structs `PascalCase`; functions `snake_case`.
- Keep prompts concise, repo-specific, and in Markdown; prefer relative paths in config.

## Testing Guidelines
- Use `cargo test` for unit coverage; place integration tests under `tests/`.
- When exercising providers, mock them (e.g., replace `codex`/`claude` in PATH) to avoid real calls in CI.
- Add regression tests for parsing (`parser/`) and discovery (`discovery/`) when changing output formats or scope logic.

## Commit & Pull Request Guidelines
- Commit messages: concise, imperative (“Add codex resume parsing”, “Fix chunk resume flag”), grouping related changes.
- PRs should include: summary of behavior change, test commands run, any config/CLI flag changes, and links to relevant issues.
- If altering provider flags/session handling, call it out explicitly for reviewers.

## Security & Configuration Tips
- Providers depend on existing CLI auth (ChatGPT/Claude sessions); never commit tokens or `~/.codex`/`~/.anthropic` contents.
- Reports and `summary.json` may contain sensitive findings—avoid committing them.
- Use `--report-dir` overrides if running in shared workspaces; keep `config.target` pointed at the repo root for correct path resolution.
