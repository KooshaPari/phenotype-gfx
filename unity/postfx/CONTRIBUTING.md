# Contributing to phenotype-postfx

Thanks for your interest in contributing to **phenotype-postfx**, part of the [Phenotype](https://github.com/KooshaPari) ecosystem of shared packages for Unity/WorldBox mods.

## AgilePlus spec mandate

All non-trivial work in this organization is tracked in **AgilePlus**. Before opening a PR for a feature or substantive change:

1. Check the [AgilePlus](https://github.com/KooshaPari/AgilePlus) spec registry for an existing spec.
2. If none exists, open one (`agileplus specify --title "<feature>" --description "<desc>"`) and link it from your PR description.
3. Trivial fixes (typos, dependency bumps, doc tweaks) do not require a spec.

## Build & test

This is a C# Unity package with a `dotnet` test suite.

```bash
git clone https://github.com/KooshaPari/phenotype-postfx.git
cd phenotype-postfx

# Build the package
dotnet build

# Run tests via Task
task test

# Or run individual test projects
dotnet test tests/PostStackSourceTests.csproj --configuration Release
dotnet test tests/PostStackVariantTests/PostStackVariantTests.csproj --configuration Release

# Lint / format checks
task lint
# or
dotnet format --verify-no-changes --verbosity diagnostic
```

Unity consumers should add the project as a package reference, matching the `phenotype-voxel`/`phenotype-water` convention.

## Branch naming

Use kebab-case prefixed by intent:

- `feat/<scope>-<short-desc>`  — new feature
- `fix/<scope>-<short-desc>`   — bug fix
- `chore/<scope>-<short-desc>` — maintenance, deps, docs
- `refactor/<scope>-<short-desc>` — internal restructuring with no behavior change

Examples:

- `feat(ssao): add multi-tap kernel`
- `fix(bloom): correct threshold scale`
- `chore(deps): bump to latest C# language version`

Commit subjects follow the same prefix style:

- `feat(api): add multi-tap kernel`
- `fix(shader): correct threshold scale`
- `chore(deps): bump language version`

## Pull request expectations

- Keep PRs focused and small; split unrelated changes.
- Ensure the build, tests, lint, and format checks above pass locally before pushing.
- Describe **what** changed and **why**. Link the AgilePlus spec, issue, or ADR.
- Expect review from `@KooshaPari` per `CODEOWNERS`; be responsive to feedback.
- Squash-merge is the default; the PR title becomes the commit subject.

## Quality gates

This repo participates in the Phenotype quality regime: zero new lint suppressions without justification, traceability to FR IDs where applicable, and 0-error CI on Linux runners. See `CLAUDE.md` / `AGENTS.md` if present for repo-specific governance.

## Reporting issues

Open a GitHub issue with reproduction steps, expected vs. actual behavior, and environment details (Unity version, target WorldBox/Phenotype mod, OS).

## Reporting security issues

Please follow the policy in `SECURITY.md`: do not open public issues for security findings; use private vulnerability reporting or email.
