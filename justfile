# hyprconf development task runner.
# Run `just` (or `just --list`) to see available recipes.

set shell := ["bash", "-uc"]

# Default recipe: list everything.
default:
    @just --list

# Build the whole workspace.
build:
    cargo build --workspace

# Run the GUI front-end.
run *ARGS:
    cargo run -p hyprconf-gui -- {{ARGS}}

# Run all tests in the workspace.
test:
    cargo test --workspace

# Lint with clippy, treating warnings as errors (matches CI).
lint:
    cargo clippy --workspace --all-targets -- -D warnings

# Format the whole workspace in place.
fmt:
    cargo fmt --all

# Verify formatting without modifying files (matches CI).
fmt-check:
    cargo fmt --all -- --check

# The full local gate, mirroring CI: format check + clippy + tests.
ci: fmt-check lint test
