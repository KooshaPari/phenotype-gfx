# unity/postfx-shaders — HLSL source of truth (preserved verbatim)

**Status (2026-06-18):** HLSL shaders from `phenotype-postfx/Runtime/Shaders/*.shader`,
copied verbatim as the source of truth for the C# edge.

## Why

Per ADR-004 (`single Rust core + thin FFI edges`), the Rust core is
engine-agnostic and only describes pass shapes (effect + shader name +
keyword + cost). The actual HLSL is engine-specific and lives here.

The Rust core references these files via `include_str!` (see
`/src/postfx/shaders.rs`), so they are also embedded in the lib's
compiled output for documentation and downstream tooling.

## Files (9)

| File                          | Upstream shader name                  | Pass effect         |
|-------------------------------|---------------------------------------|---------------------|
| `BloomPass.shader`            | `Hidden/Phenotype/BloomPass`          | Multi-pass bloom    |
| `BrpBloom.shader`             | `Hidden/Phenotype/BrpBloom`           | BRP single-pass bloom |
| `BrpACES.shader`              | `Hidden/Phenotype/BrpACES`            | BRP ACES tonemap    |
| `ColorGradingLUT.shader`      | `Hidden/Phenotype/ColorGradingLUT`    | LUT color grading   |
| `ChromaticAberration.shader`  | `Hidden/Phenotype/ChromaticAberration` | RGB-channel offset |
| `Vignette.shader`             | `Hidden/Phenotype/Vignette`           | Radial darkening    |
| `ScreenSpaceAO.shader`        | `Hidden/Phenotype/ScreenSpaceAO`      | SAO (HBAO-style) AO |
| `ScreenSpaceGI.shader`        | `Hidden/Phenotype/ScreenSpaceGI`      | SSGI                |
| `SSAOPass.shader`             | `Hidden/Phenotype/SSAOPass`           | Main SSAO pass      |

## Do not edit

These files are the canonical HLSL source. Do not modify them in-place.
If a shader change is needed, update the upstream `phenotype-postfx`
repo first, then re-copy the file here. The Rust port is in
[`/src/postfx/`](../src/postfx/).
