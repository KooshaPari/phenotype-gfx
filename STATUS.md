# Status

Last updated: 2026-06-08

## Build

GitHub Actions billing-blocked org-wide. Workflows are configured but not running.

## Quality gates

- dotnet build: local + CI (via UnityEngine stub)
- dotnet test: local + CI (xUnit + UnityEngine stub)
- dotnet format: local + CI

## Current state

- Branch: `master` (default)
- Working tree: clean
- Stashes: 0
- Open PRs: 0
- License: MIT
- Tests: xUnit project in `tests/`

## Recent changes

- Merged `chore/editorconfig-and-gitattributes` (41 commits: governance, CI, Unity stub, Taskfile.yml)
- Fixed test .csproj HintPath to use portable `$(WorldBoxManaged)`
- Added LICENSE (MIT)

## Cross-references

See `phenotype-org-governance/SUPERSEDED.md` for canonical authority.
