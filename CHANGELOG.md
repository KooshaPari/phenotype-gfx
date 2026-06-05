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
- Foundation: `.github/workflows/dotnet-build.yml` (Ubuntu, .NET 8, `dotnet build -c Release`, with a no-op stub for the Unity reference under `$(WorldBoxManaged)`).
- Foundation: `.github/dependabot.yml` (weekly updates for `github-actions`, 5-PR limit; `nuget` intentionally not enabled — see the YAML comment for the rationale and re-enable condition).
- Foundation: `.github/ISSUE_TEMPLATE/bug_report.md` and `feature_request.md`.
- Foundation: `.github/pull_request_template.md` (summary, type-of-change checklist, affected surface, testing checklist, spec/traceability, risks, related).
- Foundation: `CONTRIBUTING.md` (AgilePlus spec mandate, dotnet build command, kebab-case branch conventions, Conventional Commits, PR expectations with explicit backward-compatible public-API rule).
- Docs: tighten `README.md` license claim from "Apache-2.0 OR MIT" to MIT-only (org `LICENSE` is MIT).
- Docs: fix `CLAUDE.md` "Key Files" entry that listed `src/LodManager.cs` — the actual file in `src/` is `src/TerrainLod.cs`.
- Foundation: `CODE_OF_CONDUCT.md` (full Contributor Covenant v2.1 mirroring the org root; the org's "Reporting" section points to KooshaPari on GitHub).
- Foundation: `.github/ISSUE_TEMPLATE/config.yml` (disables blank issues, adds contact links for Security advisories, AgilePlus specs, and the Phenotype org so the private disclosure path is discoverable from the new-issue chooser).
- CI: `.github/workflows/dotnet-build.yml` Unity stub now produces a real PE-format .NET 8.0 assembly with `Vector3`/`Vector2` in the `UnityEngine` namespace, instead of a placeholder text file. The previous stub would have failed the moment a future PR added a Unity type to `src/ChunkMeshBuilder.cs` (the C# compiler opens HintPath as PE metadata). Build is fully offline (uses the dotnet SDK already on the runner).
- CI: `.github/workflows/dotnet-build.yml` build job now has `timeout-minutes: 10` (was: default 360). Bounds the blast radius of a hang — e.g. a flaky NuGet restore, a `dotnet build` deadlock, or a stub-assembly build that hangs on a transient SDK download. Clean runs finish in well under 2 minutes on a warm cache.
- CI: `.github/workflows/dotnet-build.yml` build job now `runs-on: ubuntu-24.04` (was: `ubuntu-latest`). `ubuntu-latest` is a moving target — it resolves to a different Ubuntu LTS every ~2 years. A green build today is not a guarantee of a green build next year if a future LTS brings a behavior change (apt mirror policy, dotnet SDK version, TFM compatibility check). OmniRoute already pins `ubuntu-24.04`; this matches the org convention and gives reproducible CI.
