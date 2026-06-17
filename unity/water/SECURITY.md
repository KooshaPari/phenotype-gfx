# Security Policy

## Reporting a Vulnerability

Please **do not** open a public GitHub issue for security findings. Use one of the private channels below:

1. **GitHub Security Advisories** (preferred) — open a private advisory at
   `https://github.com/Phenotype-org/phenotype-water/security/advisories/new`.
2. **Direct maintainer contact** — reach out to the repository owner listed in
   `CODEOWNERS`.

When reporting, include:

- A clear description of the issue and its impact
- Reproduction steps or a minimal PoC (call site, expected vs. actual behaviour)
- The Unity version, WorldBox install path, and the affected public API surface
  (e.g. `GerstnerWaveBank`, `FluidMesh`, `WaterLod`)
- Any `AllowUnsafeBlocks` / pointer / `Vector2`/`Vector3` boundary that the
  report touches, since those paths are the most likely source of memory-safety
  issues in this package

## Scope

The following are in scope:

- Public API surface of `Phenotype.Water` that ships to consumers as a sibling
  project reference
- Build-time or load-time risks that affect consumers (e.g. unsigned binaries,
  unexpected `Write*` paths, environment variable leaks)
- Memory-safety issues around `<AllowUnsafeBlocks>true</AllowUnsafeBlocks>` in
  `phenotype-water.csproj`

The following are **out of scope**:

- Vulnerabilities in the Unity runtime, WorldBox, or any consumer mod — report
  those to their maintainers
- Issues that only exist on a fork or a local modification

## Response

| Stage               | Target              |
| ------------------- | ------------------- |
| Acknowledgment      | 7 calendar days     |
| Triage & Assessment | 14 calendar days    |
| Patch Release       | Best-effort, prioritised by severity |

A finding is acknowledged when a Security Advisory draft is opened and you are
added as a collaborator on it. Status updates will land on the same advisory
thread.

## Supported Versions

| Version            | Support Status |
| ------------------ | -------------- |
| `main` (HEAD)      | ✅ Active      |
| Tagged releases    | ✅ Best-effort  |
| Older scaffolds    | ❌ Unsupported  |

## Notes

This package is a shared Unity runtime library consumed by sibling Phenotype
mods. The attack surface is dominated by malformed wave parameters, bad LOD
ranges, and pointer arithmetic in the `GerstnerWaveBank` evaluator. When in
doubt, file the report — the maintainer can downgrade scope after triage.
