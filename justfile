set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

# List available recipes
default:
    just --list

# Start development watch mode
dev:
    cargo watch -x "check --workspace --all-features" -x "test --workspace --all-features"

# Build release artifacts
build:
    cargo build --workspace --all-features --release

# Run the test suite
test:
    cargo test --workspace --all-features

# Run linter
lint:
    cargo clippy --workspace --all-features --all-targets -- -D warnings

# Apply formatter
fmt:
    cargo fmt --all

# Remove build artifacts
clean:
    cargo clean
