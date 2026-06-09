# Status

Last updated: 2026-06-08

## Build

GitHub Actions billing-blocked org-wide. Workflows are configured but not running.

## Quality gates

- dotnet build: local + CI
- dotnet test: local + CI (tests/ added 2026-06-08)
- dotnet format: local + CI

## Current state

- Branch: `master` (default)
- Working tree: clean
- Stashes: 0
- Open PRs: 0
- License: MIT
- Tests: xUnit project in `tests/` with coverage for LodBase, TerrainLod, HeightField

## Recent changes

- Fixed README stale license claim (LICENSE exists)
- Added `tests/` with `phenotype-terrain.tests.csproj` and xUnit coverage
- Added test step to CI workflow
- Updated Taskfile.yml `test` task to run actual tests

## Cross-references

See `phenotype-org-governance/SUPERSEDED.md` for canonical authority.
