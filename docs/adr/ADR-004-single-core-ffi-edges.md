# ADR-004: Single Rust Core + Thin FFI Edges

**Status:** Accepted (2026-06-16, user-locked)

**Supersedes:** ADR-001 (sister-repos), monorepo PR #3

## Context

phenotype-gfx needs to serve multiple consumers: WSM3D (C#/Unity), web (TypeScript/WASM), and Civis (Bevy/Rust). Previous approaches (ADR-001 sister-repos, PR #3 monorepo) would duplicate algorithm logic across languages.

## Decision

**ONE max-optimal CORE (Rust)** holds all gfx algorithms (voxel, LOD, streaming, postfx, water, voxelizer). NO duplicated logic.

**THIN EDGES (bindings, NOT reimplementations):**
- C ABI via cbindgen -> C# P/Invoke -> WSM3D
- wasm-bindgen -> TS/npm -> web consumers
- Direct Rust dep -> Civis/phenotype-voxel

## Rationale

- Rust chosen: phenotype-voxel + Civis already Rust; strongest FFI ecosystem
- Can later drop SIMD hotpath to Zig/Mojo behind FFI without changing consumers
- Zero duplication: algorithms live in Rust core exactly once

## Structure

```
phenotype-gfx/          (single Rust crate)
├── src/
│   ├── lib.rs          (module tree)
│   ├── voxel.rs        (voxel kernel -- from phenotype-voxel + Civis)
│   ├── lod.rs          (LOD system -- from Civis lod.rs)
│   ├── streaming.rs    (streaming -- from Civis scale_budget.rs)
│   ├── postfx.rs       (postfx -- PORT from WSM3D C# PostStack.cs)
│   ├── water.rs        (water -- from phenotype-water + Civis)
│   └── voxelizer.rs    (voxelizer -- PORT from WSM3D C# SpriteVoxelizer.cs)
└── bindings/
    └── c_api.rs        (C-ABI stub via cbindgen -- future)
```

## Transition Plan

1. **WSM3D**: Port C# postfx/voxelizer LOGIC into Rust core; C# becomes thin P/Invoke wrapper calling C-ABI
2. **Web**: Expose via wasm-bindgen (future step)
3. **Civis**: Add as Rust dependency directly

## Superseded Decisions

- **ADR-001 (sister-repos):** No longer valid. Sister-repo pattern duplicates logic.
- **Monorepo PR #3:** No longer valid. Monorepo with multi-language impls duplicates logic. Closed with comment referencing ADR-004.
