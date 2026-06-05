# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Foundation: `.editorconfig`, `.gitattributes` (LF normalize, `.meta` → `unityyamlmerge`).
- Foundation: `AGENTS.md` (local agent governance, do/don't rules, sibling package consumers).
- Foundation: `CLAUDE.md` (Claude-specific entry point mirroring the McpKit stack template).
- Foundation: `SECURITY.md` (private disclosure via GitHub Security Advisories, response targets).
- Foundation: `CODEOWNERS` (default reviewer: `@KooshaPari`).
- Foundation: `.github/workflows/dotnet-test.yml` (Ubuntu, .NET 8, runs the existing xunit test project `tests/phenotype-water.tests.csproj`).
- Foundation: `.github/dependabot.yml` (weekly updates for `github-actions`, 5-PR limit; `nuget` intentionally not enabled — see the YAML comment for the rationale and re-enable condition).
- CI: `.github/dependabot.yml` now enables the `nuget` ecosystem scoped to `directory: "/tests"`. The previous justification said "the `phenotype-water.csproj` declares zero `<PackageReference>` items" — that is still true for the library, but the *test* project declares three (xunit 2.9.3, xunit.runner.visualstudio 2.8.2, Microsoft.NET.Test.Sdk 17.12.0). Without dependabot these rot on the pinned version and miss security-update PRs (xunit 2.9.x is current; older 2.4.x had a known transitive vulnerability in its `System.Text.Json` chain). The scope is `/tests` because the library has zero NuGet deps; if a NuGet dep is ever added to the library, add a second `nuget` block with `directory: "/"` so library and test updates flow independently.
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
