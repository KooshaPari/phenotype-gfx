# AGENTS.md — phenotype-water

This file governs work inside the `phenotype-water` repository.

## Identity

`phenotype-water` is a shared Unity water package for Phenotype-org mods targeting Unity / WorldBox. It provides a Gerstner-wave water system with `GerstnerWaveBank`, `FluidMesh`, and `WaterLod` for camera-distance LOD. Designed for net48 Unity builds.

Do not apply parent shelf instructions (e.g. `/Users/kooshapari/CodeProjects/Phenotype/repos/AGENTS.md` or `~/.claude/AGENTS.md`) unless explicitly referenced. Work from this directory and treat paths as local to `phenotype-water`.

## Quick Links

- **Local CLAUDE.md:** Not present; this AGENTS.md is the source of truth for agent behavior in this repo.
- **Phenotype org governance:** `/Users/kooshapari/CodeProjects/Phenotype/repos/CLAUDE.md` (consult when touching cross-repo contracts).
- **Global agent guidance:** `~/.claude/AGENTS.md` (consult for global defaults).
- **AgilePlus work tracking:** `cd /repos/AgilePlus && agileplus <command>` — required for non-trivial work per the CONTRIBUTING mandate.
- **Sibling shared packages:** `phenotype-voxel`, `phenotype-terrain` (sibling Phenotype mod packages).

## Working Conventions

- **Branch naming:** `<type>/<topic>` in kebab-case, conventional commits. See `CONTRIBUTING.md`.
- **PR expectations:** Use the repository's pull_request_template (when present). Each PR links an AgilePlus spec, includes a short rationale, and notes any consumer-side impact on `phenotype-voxel` / `phenotype-terrain`.
- **Quality gates:** `dotnet build phenotype-water.slnx` succeeds; consumers recompile against the changed surface. Unity consumers regenerate `.meta` and resolve via sibling project reference.
- **Stack:** C# / .NET (Microsoft.NET.Sdk), targeting net48. Edit `.editorconfig` to relax linting on Unity auto-generated `.meta` files.
- **Traceability:** Substantive work links FR IDs (e.g. `FR-WATER-GERSTNER-001`) or an ADR. XML doc comments on public API surface.
- **Security disclosures:** Follow `SECURITY.md`; never open public issues for security findings.

## Do / Don't

- **Do** keep public API changes backward-compatible. Add a new method/overload rather than mutating signatures; deprecate before removing.
- **Do** keep the package consumer-friendly: prefer pure C# / .NET, no Unity Editor-specific dependencies leaking into the runtime API.
- **Do** update `README.md` and any usage examples when the public surface changes.
- **Do** document the wave parameters and LOD tier boundaries in code comments so consumers can tune safely.
- **Don't** hand-roll Gerstner / water LOD code in consumer packages; surface it here and have them reference this project.
- **Don't** introduce Editor-only API into the runtime assembly; it bloats consumers.
- **Don't** commit binary `.meta` or build artifacts; `.gitattributes` marks them as binary / merge-only.

## Status

This AGENTS.md is living governance for `phenotype-water`. Update it when the working conventions change, and link any new tooling, scripts, or process notes here.
