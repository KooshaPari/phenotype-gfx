# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- CI: `.github/scorecard.yml` OpenSSF Scorecard policy mirroring `phenotype-terrain`. Pins Binary-Artifacts, Branch-Protection, Code-Review, Dangerous-Workflow, Dependency-Update-Tool, Pinned-Dependencies, SAST, Security-Policy, and Token-Permissions checks to the same thresholds terrain uses, so a green run on terrain implies parity on water.
- Foundation: `.editorconfig`, `.gitattributes` (LF normalize, `.meta` → `unityyamlmerge`).
- Foundation: `AGENTS.md` (local agent governance, do/don't rules, sibling package consumers).
- Foundation: `CLAUDE.md` (Claude-specific entry point mirroring the McpKit stack template).
- Foundation: `SECURITY.md` (private disclosure via GitHub Security Advisories, response targets).
- Foundation: `CODEOWNERS` (default reviewer: `@KooshaPari`).
- Foundation: `.github/workflows/dotnet-test.yml` (Ubuntu, .NET 8, runs the existing xunit test project `tests/phenotype-water.tests.csproj`).
- Foundation: `.github/dependabot.yml` (weekly updates for `github-actions`, 5-PR limit; `nuget` intentionally not enabled — see the YAML comment for the rationale and re-enable condition).
- CI: `.github/dependabot.yml` now enables the `nuget` ecosystem scoped to `directory: "/tests"`. The previous justification said "the `phenotype-water.csproj` declares zero `<PackageReference>` items" — that is still true for the library, but the *test* project declares three (xunit 2.9.3, xunit.runner.visualstudio 2.8.2, Microsoft.NET.Test.Sdk 17.12.0). Without dependabot these rot on the pinned version and miss security-update PRs (xunit 2.9.x is current; older 2.4.x had a known transitive vulnerability in its `System.Text.Json` chain). The scope is `/tests` because the library has zero NuGet deps; if a NuGet dep is ever added to the library, add a second `nuget` block with `directory: "/"` so library and test updates flow independently.
- Docs: `AGENTS.md` "Quick Links" no longer claims "Local CLAUDE.md: Not present". A `CLAUDE.md` is in fact at the repo root (foundation governance stack). The stale claim would have routed an agent away from `CLAUDE.md` and into `AGENTS.md` as a stand-in, which defeats the layered-governance design.
- Docs: removed phantom `phenotype-voxel` references from `AGENTS.md`, `CLAUDE.md`, and `CONTRIBUTING.md`. `https://github.com/Phenotype-org/phenotype-voxel` returns HTTP 404 — the package does not exist. The reference was aspirational leftover from a draft. The in-repo sibling `phenotype-terrain` is preserved (it is real and water depends on it).
- Docs: `CLAUDE.md` "Key Commands" comment for `dotnet test` no longer says "# when tests exist" — there are 39 [Fact]/[Theory] tests across three test files in `tests/`, so the qualifier was misleading. Replaced with "# runs the xunit tests under tests/", which matches what the command does.
- Foundation: `.github/ISSUE_TEMPLATE/bug_report.md` and `feature_request.md`.
- Foundation: `.github/pull_request_template.md` (summary, type-of-change checklist, affected surface, testing checklist, spec/traceability, risks, related).
- Foundation: `CONTRIBUTING.md` (AgilePlus spec mandate, dotnet build + dotnet test commands, kebab-case branch conventions, Conventional Commits, PR expectations with explicit backward-compatible public-API rule and the "changes to the unsafe path must come with a test" rule).
- Docs: expand `README.md` to mirror the phenotype-terrain layout — Build section with the `WorldBoxManaged` env var and `dotnet build`/`dotnet test`, Consuming section with the project-reference XML and a note that `tests/phenotype-water.tests.csproj` is not a consumer dependency, and License pointing at the in-repo `LICENSE`.
- Docs: `README.md` "License" section now reflects the actual state — no `LICENSE` file has been committed, the link to `./LICENSE` is a dead reference. Replaced the false link with an honest statement of intent (MIT) plus a clear note that the file is missing and needs to land before the first public release.
- Foundation: `CODE_OF_CONDUCT.md` (full Contributor Covenant v2.1 mirroring the org root; the org's "Reporting" section points to KooshaPari on GitHub).
- Foundation: `.github/ISSUE_TEMPLATE/config.yml` (disables blank issues, adds contact links for Security advisories, AgilePlus specs, and the Phenotype org so the private disclosure path is discoverable from the new-issue chooser).
- CI: `.github/workflows/dotnet-test.yml` now fails fast with a clear `::error::` message if `WorldBoxManaged` is unset. The library and test project reference `UnityEngine.CoreModule.dll` with a hardcoded absolute Windows path, which is invalid on a Linux runner. The right long-term fix is to make the `.csproj` HintPath portable (use `$(WorldBoxManaged)/UnityEngine.CoreModule.dll` like `phenotype-terrain` already does) and add a `Directory.Build.targets` that substitutes the variable; that is a substantive contract change reserved for the package owner. This commit just makes the broken state visible in the Actions UI instead of as a noisy MSBuild trace.
- CI: `.github/workflows/dotnet-test.yml` test job now has `timeout-minutes: 10` (was: default 360). Bounds the blast radius of a hang — e.g. an xunit test that infinite-loops, a `dotnet test` deadlock, or a flaky NuGet restore. Clean runs (which exercise several hundred assertions across `GerstnerWaveBank`, `WaterLod`, and `FluidMesh`) finish in well under a minute on a warm cache.
- CI: `.github/workflows/dotnet-test.yml` test job now `runs-on: ubuntu-24.04` (was: `ubuntu-latest`). `ubuntu-latest` is a moving target — it resolves to a different Ubuntu LTS every ~2 years. A green build today is not a guarantee of a green build next year if a future LTS brings a behavior change (apt mirror policy, dotnet SDK version, TFM compatibility check). OmniRoute already pins `ubuntu-24.04`; this matches the org convention and gives reproducible CI.
- CI: `.github/workflows/dotnet-test.yml` `pull_request.branches` now matches `push.branches` (master, main, chore/**, feat/**, fix/**, refactor/**). Previously, a PR opened directly against a feature branch (e.g. a follow-up review commit to `chore/editorconfig-and-gitattributes`) would not trigger CI via the pull_request event — the gap was masked for the common "commit on feature branch, then PR to master/main" flow, but any PR targeting a feature branch directly would have skipped CI.
- Refactor: derive `WaterLod` from shared `LodBase`, removing 61 lines of duplicated LOD logic.
- CI: replace the fail-fast `WorldBoxManaged` check with a `src/UnityEngineStubs.cs` stub compilation step so CI can build and test on Linux without a Unity installation; add `dotnet format --verify-no-changes` as a quality gate.
- Tests: replace hardcoded Windows `WorldBoxManaged` path with portable `$(WorldBoxManaged)` HintPath in `.csproj`.
- Docs: `STATUS.md` with build state, quality gates, and cross-references.
- Build: `NuGet.config` for package source configuration.
- Build: `UnityEngineStubs.cs` fallback for CI and cross-platform builds — compiles a minimal UnityEngine stub assembly when the real `UnityEngine.CoreModule.dll` is not available.
- LOD: setters on `WaterLod` distance properties (previously read-only) and `Vector2.zero` stub for cross-platform compilation.
- CI: update `dotnet-test` workflow to use `src/UnityEngineStubs.cs` for the Unity stub instead of an inline heredoc.
- CI: fix heredoc in the Unity stub step to correctly expand the repository path.
- CI: extract the inline UnityEngine.CoreModule stub from the workflow YAML into a shared `.github/scripts/stub-unity-engine.sh` script.
- CI: switch runner to `ubuntu-latest` to fix runner provisioning issues, then re-pin to `ubuntu-24.04` for reproducible builds.
- CI: standardize runner to `ubuntu-24.04` and pin third-party actions to immutable SHA hashes.
- Tooling: `Taskfile.yml` with SSOT recipes for `build`, `lint`, `test`, and `validate`.
- Tests: `GerstnerWaveBankTests` with xUnit `[Fact]` and `[Theory]` assertions for wave synthesis validation.
- Tests: `FluidMeshStressTests` for high-density mesh stress validation.
- CI: Dependabot configuration (`.github/dependabot.yml`) for cargo, nuget, and github-actions ecosystems with PR limit 5.
- Refactor: move `WaterMesh` and `WaterLod` into the `Phenotype.Water.Rendering` namespace to separate the rendering pipeline from the wave simulation domain.
- Refactor: extract the full water rendering pipeline into the `Phenotype.Water.Rendering` namespace; fix culled mesh return path; resolve UnityEngine stub conflicts in CI.
- Tests: fix test project build for cross-platform compilation by resolving UnityEngine reference conflicts.
- Docs: align `CONTRIBUTING.md` with the `phenotype-terrain` template for consistency across the org.
- Docs: comprehensive XML API documentation comments for all public types (`GerstnerWaveBank`, `FluidMesh`, `WaterLod`, `WaterRenderer`, etc.).

### Changed
- CI: `.github/workflows/dotnet-test.yml` re-pins the test job `runs-on` from `ubuntu-latest` to `ubuntu-24.04`. The repo had previously been pinned to `ubuntu-24.04` (commit `db01906`) then switched to `ubuntu-latest` to work around a transient runner-provisioning issue (commit `c377e2a`); the workaround stuck even after the upstream issue resolved. `ubuntu-latest` is a moving target — it resolves to a different Ubuntu LTS every ~2 years, so a green build today is not a guarantee of a green build next year. `phenotype-terrain` already pins `ubuntu-24.04`; this restores org parity and gives reproducible CI.

### Hygiene
- `.gitignore` expanded to cover: `_restore/` (scratch dir for `dotnet restore --packages` overrides), `TestResults/` and `coverage/` and `*.coverage` (xUnit / dotnet test artifacts), `.DS_Store` and `Thumbs.db` (macOS / Windows shell metadata), `*.swp` and `*.swo` (Vim swap files). The pre-existing `bin/`, `obj/`, `*.user`, `*.suo`, `.vs/` entries are preserved. Mirrors the `phenotype-terrain` pattern of an explicit, well-commented ignore list so contributors do not have to discover scratch paths from the `.gitignore` in a different repo.
