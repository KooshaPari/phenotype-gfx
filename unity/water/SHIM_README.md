# unity/water — thin C# shim (superseded)

**Status (2026-06-18):** The actual water logic now lives in
`/src/water/` (Rust). This directory retains the original C# source tree
(`src/`, `tests/`, `phenotype-water.csproj`, etc.) for reference and as a
build target for the C# edge, but **the C# code should NOT be edited**.

## Why

Per ADR-004 (`single Rust core + thin FFI edges`), the engine edge in C# is
a thin P/Invoke shim. The real `GerstnerWaveBank`, `FluidMesh`, `WaterLod`,
material/serialization port logic, and the 9 test suites were ported to
`phenotype-gfx/src/water/` on 2026-06-18 (L5-111).

`WaterMaterial` / `WaterRenderer` / `WaterShader` are marked
`#[deprecated]` pass-throughs in the Rust core; their real per-pass
config types are now first-class (`GerstnerWaveBank` itself for waves,
`WaterLod` for LOD).

## What to do here

If you need to add or change a C# entry point for a Unity asset / MonoBehaviour,
add it here as a thin wrapper that calls into the Rust core via P/Invoke.
Do not re-implement the port logic in C#.

## Source repos

- Upstream C#: <https://github.com/KooshaPari/phenotype-water> (now archived)
- Rust port: [`/src/water/`](../src/water/) in this crate
