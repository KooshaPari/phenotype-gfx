# Status

Last updated: 2026-06-08

## Build

- GitHub Actions: configured (billing-blocked org-wide)
- Local: `just test` or `task test` runs xUnit tests via .NET 8.0
- Unity package: drop `Runtime/` into Assets (Built-In Render Pipeline, 2021.3+)

## Quality gates

- `just lint` / `task lint` — dotnet format verification
- `just test` / `task test` — xUnit source-signature + shader-variant tests
- `just validate` / `task validate` — package.json + shader existence checks

## Current state

- Branch: `main` (default)
- Working tree: clean
- Stashes: 0
- Open PRs: 0
- License: MIT (added 2026-06-08)

## Recent changes

- Added LICENSE (MIT)
- Fixed Taskfile.yml to reference actual test csproj files
- CI workflow uses SHA-pinned actions, `just test`, ubuntu-24.04

## Cross-references

See `phenotype-org-governance/SUPERSEDED.md` for canonical authority.
