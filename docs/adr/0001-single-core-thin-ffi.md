# ADR-0001: Single Rust Core + Thin FFI Edges (User-Locked Architecture)

## Decision
ONE max-optimal Rust core (phenotype-gfx) holds all graphics algorithms ONCE.
THIN FFI edges (C-ABI, wasm-bindgen) expose to consumers without duplicating logic.

## Rationale
- Rust chosen: phenotype-voxel + Civis already Rust, strongest FFI story
- NO multi-language reimplementations (sister-repos ADR-001: SUPERSEDED)
- Future: can drop SIMD hotpath to Zig/Mojo behind same FFI boundary without changing consumers

## Core Modules
- voxel: voxel kernel, storage, meshing (from phenotype-voxel + Civis)
- lod: LOD system, frustum culling, render planning (from Civis)
- streaming: chunk window, ring LOD, eviction (from Civis)
- postfx: post-processing pipeline (SSAO→SSGI→Bloom→ACES→LUT, from WSM3D C# logic ported to Rust)
- water: Gerstner waves, fluid mesh (from phenotype-water + Civis)
- voxelizer: sprite voxelizer (from WSM3D C# logic ported to Rust)

## FFI Edges (Future)
- C-ABI: cbindgen + C# P/Invoke shim (WSM3D consumes via binding)
- wasm-bindgen: TS/npm for web consumers

## Supersedes
- ADR-001 (sister-repos): No longer valid; consolidated to single core
- Monorepo PR #3: No longer valid; single core replaces

## Next Steps (Task #2)
1. (This PR) Scaffold core crate + module stubs
2. (PR2) Fold Civis Rust modules (lod.rs, material_pbr.rs, scale_budget.rs)
3. (PR3+) Port WSM3D C# logic (postfx, voxelizer) into Rust core
4. (Future) Add cbindgen C-ABI wrapper + C# shim for WSM3D
