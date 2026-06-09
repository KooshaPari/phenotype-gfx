# Contributing to phenotype-water

Thanks for your interest in contributing to **phenotype-water**, part of the [Phenotype](https://github.com/KooshaPari) ecosystem of shared packages for Unity/WorldBox mods.

## AgilePlus spec mandate

All non-trivial work in this organization is tracked in **AgilePlus**. Before opening a PR for a feature or substantive change:

1. Check the [AgilePlus](https://github.com/KooshaPari/AgilePlus) spec registry for an existing spec.
2. If none exists, open one (`agileplus specify --title "<feature>" --description "<desc>"`) and link it from your PR description.
3. Trivial fixes (typos, dependency bumps, doc tweaks) do not require a spec.

## Build & test

This is a C# / .NET shared Unity package (net48). The library references `UnityEngine.CoreModule.dll` under `$(WorldBoxManaged)`. A test project lives under `tests/phenotype-water.tests.csproj` and uses xunit.

```bash
# Local build (Windows or any host with the WorldBox Managed/ directory)
$env:WorldBoxManaged = "C:/Program Files (x86)/Steam/steamapps/common/worldbox/worldbox_Data/Managed"
dotnet build phenotype-water.slnx -c Release
dotnet test  tests/phenotype-water.tests.csproj -c Release

# CI build (Linux runner without WorldBox) — uses a no-op stub for the Unity reference
# See .github/workflows/dotnet-test.yml
```

The CI workflow at `.github/workflows/dotnet-test.yml` restores, builds, and runs the xunit test project on every push, PR, and manual dispatch.

## Branch naming

Use kebab-case prefixed by intent:

- `feat/<scope>-<short-desc>`     — new feature
- `fix/<scope>-<short-desc>`      — bug fix
- `chore/<scope>-<short-desc>`    — tooling, deps, infra
- `docs/<scope>-<short-desc>`     — docs only
- `refactor/<scope>-<short-desc>` — non-behavioral change

## Commit messages

Follow [Conventional Commits](https://www.conventionalcommits.org/). Examples:

- `feat(gerstner): add per-wave steepness clamp`
- `fix(fluid-mesh): correct normal recomputation on LOD transition`
- `chore(tests): add xunit regression for wave bank degeneracy`

If a `commitlint.config.*` exists in the repo, it is enforced; otherwise the convention above is the floor.

## Pull request expectations

- Keep PRs focused and small; split unrelated changes.
- **Public API changes must be backward-compatible.** Add new methods/overloads rather than mutating signatures; deprecate before removing. The in-repo consumer is `phenotype-terrain` (this package depends on it); downstream consumers are end-user Phenotype water mods.
- `phenotype-water.csproj` has `<AllowUnsafeBlocks>true</AllowUnsafeBlocks>`: any change that touches the unsafe path must come with a test under `tests/`.
- Ensure `dotnet build` and `dotnet test` are green before pushing.
- Describe **what** changed and **why**. Link the AgilePlus spec, issue, or ADR.
- Touched Unity surfaces: refresh screenshots in `README.md` and verify the test scene renders without warnings.
- Expect review from `@KooshaPari` per `CODEOWNERS`; be responsive to feedback.
- Squash-merge is the default; the PR title becomes the commit subject.

## Quality gates

This repo participates in the Phenotype quality regime: zero new lint suppressions without justification, traceability to FR IDs where applicable, and 0-error CI on Linux runners. See `CLAUDE.md` / `AGENTS.md` if present for repo-specific governance.

## Reporting issues

Open a GitHub issue with reproduction steps, expected vs. actual behavior, and environment details (Unity version, WorldBox install path, dotnet SDK version, consumer project, the public API surface touched).

## Security

Do not open public issues for security findings. See `SECURITY.md` for the private disclosure path. Pay particular attention to memory-safety issues around `<AllowUnsafeBlocks>` and the `GerstnerWaveBank` evaluator.
