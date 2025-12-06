default:
    @just --list

# Run all reviewers
run *ARGS:
    cargo run -- run {{ARGS}}

# Create issues from reports
issue *ARGS:
    cargo run -- issue {{ARGS}}

# Print JSON schema
schema:
    cargo run -- schema > polyrev.schema.json

# Dry run (no provider calls)
dry-run:
    cargo run -- run --config examples/polyrev.yaml --dry-run

# CI mode (fail on critical)
ci:
    cargo run -- run --config examples/polyrev.yaml --fail-on-critical

# Run tests
test:
    cargo test

# Run tests with output
test-verbose:
    cargo test -- --nocapture

# Build release binary
build:
    cargo build --release

# Install locally
install:
    cargo install --path .

# Lint
lint:
    cargo clippy -- -D warnings

# Format check
fmt:
    cargo fmt --check

# Format fix
fmt-fix:
    cargo fmt

# Check (fast compile check)
check:
    cargo check

# Clean build artifacts
clean:
    cargo clean

# Generate and validate schema
validate-schema:
    cargo run -- schema > polyrev.schema.json
    @echo "Schema generated: polyrev.schema.json"
