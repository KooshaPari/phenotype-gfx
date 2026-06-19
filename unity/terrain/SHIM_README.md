# unity/terrain — thin C# shim (superseded)

**Status (2026-06-18):** The actual terrain logic now lives in
`/src/terrain/` (Rust). This directory retains the original C# source tree
(`src/`, `tests/`, `phenotype-terrain.csproj`, etc.) for reference and as a
build target for the C# edge, but **the C# code should NOT be edited**.

## Why

Per ADR-004 (`single Rust core + thin FFI edges`), the engine edge in C# is
a thin P/Invoke shim. The real `HeightField`, `ChunkMeshBuilder`,
`TerrainLod`, material/serialization port logic, and the 7 test suites
were ported to `phenotype-gfx/src/terrain/` on 2026-06-18 (L5-110).

## What to do here

If you need to add or change a C# entry point for a Unity asset / MonoBehaviour,
add it here as a thin wrapper that calls into the Rust core via P/Invoke.
Do not re-implement the port logic in C#.

## Source repos

- Upstream C#: <https://github.com/KooshaPari/phenotype-terrain> (now archived)
- Rust port: [`/src/terrain/`](../src/terrain/) in this crate
