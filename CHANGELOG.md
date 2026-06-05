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
- Foundation: `.github/dependabot.yml` (weekly updates for `github-actions` and `nuget`, 5-PR limit).
- Foundation: `.github/ISSUE_TEMPLATE/bug_report.md` and `feature_request.md`.
- Foundation: `.github/pull_request_template.md` (summary, type-of-change checklist, affected surface, testing checklist, spec/traceability, risks, related).
- Foundation: `CONTRIBUTING.md` (AgilePlus spec mandate, dotnet build command, kebab-case branch conventions, Conventional Commits, PR expectations with explicit backward-compatible public-API rule).
- Docs: tighten `README.md` license claim from "Apache-2.0 OR MIT" to MIT-only (org `LICENSE` is MIT).
- Docs: fix `CLAUDE.md` "Key Files" entry that listed `src/LodManager.cs` — the actual file in `src/` is `src/TerrainLod.cs`.
- Foundation: `CODE_OF_CONDUCT.md` (full Contributor Covenant v2.1 mirroring the org root; the org's "Reporting" section points to KooshaPari on GitHub).
