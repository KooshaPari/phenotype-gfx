# justfile for phenotype-postfx
# SSOT recipes are in Taskfile.yml.
# This justfile exists because the org's CI uses `just` for consistency
# across all 8 active repos. It delegates to the underlying Taskfile.

set shell := ["bash", "-uc"]

# Default recipe — show available tasks
default:
    @just --list

# Run all tests via task (the project's SSOT)
test:
    @echo "Delegating to Taskfile.yml: test"
    @task test

# Build via task
build:
    @echo "Delegating to Taskfile.yml: build"
    @task build

# Lint via task
lint:
    @echo "Delegating to Taskfile.yml: lint"
    @task lint

# Format via task
format:
    @echo "Delegating to Taskfile.yml: format"
    @task format

# Full quality gate
quality:
    @echo "Delegating to Taskfile.yml: quality"
    @task quality
