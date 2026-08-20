# DESIGN.md — phenotype-gfx

## Overview

**phenotype-gfx** is a polyglot graphics SDK providing voxel rendering, terrain, and water simulation primitives. It exposes both a Rust core library and Unity C# bindings.

## Architecture

```
phenotype-gfx/
├── crates/
│   └── phenotype-voxel/ # Compatibility shim re-exporting the core voxel kernel (ADR-004)
├── src/
│   ├── voxel/           # Voxel kernel: chunk storage, meshing (cubic, greedy), materials
│   ├── lod/             # LOD system: frustum culling, chunk render planning
│   ├── streaming/       # Streaming window: ring-based chunk lifecycle, eviction
│   ├── water/           # Water simulation: Gerstner waves, fluid mesh
│   ├── postfx/          # Post-processing: SSAO, SSGI, Bloom, ACES, LUT
│   ├── terrain/         # Terrain: height field, chunk mesh builder
│   └── voxelizer/       # Sprite voxelizer: voxel-to-sprite rendering
├── examples/            # Sample projects
└── docs/                # API reference + design docs
```

## Key Design Decisions

1. **Rust core + C# bindings** — performance-critical rendering in Rust, Unity integration via P/Invoke FFI
2. **Dual license (Apache-2.0 OR MIT)** — maximum compatibility for game studios and open-source projects
3. **Crate-per-feature** — each rendering primitive is an isolated crate for independent compilation and testing

## Data Flow

```
Unity C# → FFI call → gfx-core (Rust) → mesh generation → GPU buffer → Unity rendering pipeline
```

## Non-Goals

- Full game engine (use Unity/Godot for that)
- Network replication (handled by phenotype-infra)
- Editor tooling (out of scope for v0.1)

## Status

- v0.1.0 — Initial release with voxel + terrain primitives
- v0.2.0 (planned) — Water simulation + shader pipeline

## References

- [phenotype-infra DESIGN.md](../phenotype-infra/DESIGN.md) (template source)
- [Cargo.toml](./Cargo.toml) (dependency graph)
