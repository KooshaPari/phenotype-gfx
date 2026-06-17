# ADR-0001: Single-Core Rust Crate with Thin FFI Edges

**Date**: 2026-06-17
**Status**: ACCEPTED — locked, non-negotiable
**Supersedes**: ADR-001 (sister-repos / multi-repo structure), ADR-002 (postfx C# copy)
**Deciders**: Apex (KooshaPari)

---

## Context

`phenotype-gfx` is the shared graphics/rendering substrate for:

- **Civis** — Bevy 0.18 (Rust, DX12/WGSL); needs voxel SVO, mesher, LOD, streaming
- **WSM3D** — Unity 2022.3 BRP (C#, D3D11); needs terrain, water, LOD, postfx, voxelizer
- **Web** — TS/npm; needs postfx shaders as ES module

Prior proposals (sister-repos, monorepo with parallel per-language impls, postfx C# copy)
all duplicated algorithm logic across language boundaries. That is the core problem: when
an algorithm lives in two places, they diverge, bugs are fixed in one and not the other,
and the substrate stops being a substrate.

---

## Decision

**ONE max-optimal CORE** holding all gfx algorithms exactly once, with **THIN FFI/SDK
edges** per consumer. Not sister-repos. Not a monorepo of parallel per-language impls.

```
phenotype-gfx/          ← this repo
├── Cargo.toml          (single crate, not a workspace)
├── src/
│   ├── lib.rs
│   ├── voxel.rs        ← SVO kernel, dirty-chunk queue, mesher
│   ├── lod.rs          ← LOD tier selection, scale-budget
│   ├── streaming.rs    ← chunk streaming, prefetch ring
│   ├── postfx.rs       ← SSAO→SSGI→Bloom→ACES→LUT (5-pass)
│   ├── water.rs        ← Gerstner waves, surface meshing
│   └── voxelizer.rs    ← mesh-to-voxel conversion
└── docs/adr/           ← here

Binding crates (separate repos or feature-gated crates, thin):
  phenotype-gfx-ffi/   ← cbindgen C-ABI → C# P/Invoke shim (WSM3D)
  phenotype-gfx-wasm/  ← wasm-bindgen → TS/npm (web)
```

### Rules

1. **No algorithm duplication.** C# (WSM3D) and TS (web) consumers get thin bindings
   that call into this Rust core. They do NOT reimplement the algorithm.
2. **WSM3D C# postfx logic is PORTED to Rust** (`src/postfx.rs`), not copied. The C#
   file becomes a thin P/Invoke shim. Same for SpriteVoxelizer → `src/voxelizer.rs`.
3. **Fold order**: intake low-coupling Civis modules first (lod.rs, scale_budget.rs,
   material_pbr.rs from `civis-platform-wt/crates/voxel/src/`), then port WSM3D logic.
4. **No `cargo` runs required** for stub scaffolds — trivially valid Rust; CI gates on PR.

---

## Why This Beats the Prior Options

| Concern | Sister-repos (ADR-001) | C# copy (ADR-002) | Single-core (this ADR) |
|---------|----------------------|-------------------|------------------------|
| Algorithm duplication | Yes (3 copies) | Yes (C# + Rust) | No — one copy |
| Consumer isolation | Good | Good | Good (thin binding crates) |
| Bug fix propagation | 3 PRs required | 2 PRs required | 1 PR, all consumers benefit |
| Rust ecosystem (Civis) | Native | Requires FFI | Native |
| C# consumer (WSM3D) | Native C# | Native C# | Thin P/Invoke shim (minor overhead) |
| Web consumer | TS native | N/A | wasm-bindgen (standard) |

---

## Superseded Decisions

- **ADR-001 (docs/ARCHITECTURE.md, "Option A — Multi-Repo Sister Projects")**: SUPERSEDED.
  Rationale was build isolation; the real cost was algorithm duplication across repos.
- **ADR-002 (any plan to copy WSM3D C# postfx/voxelizer as-is)**: SUPERSEDED.
  C# code is ported to Rust; C# wrapper calls P/Invoke into the Rust core.

---

## Consequences

**Positive:**
- Single source of truth for every graphics algorithm
- Civis gets native Rust with zero FFI overhead
- WSM3D gets the same algorithm through a thin shim; C# layer stays minimal
- Bug fixes and optimizations propagate to all consumers via a single PR

**Negative:**
- WSM3D contributors must understand the P/Invoke boundary when adding features
- Initial port effort: WSM3D postfx (5-pass) and SpriteVoxelizer must be rewritten
  in Rust (not just copied)

**Neutral:**
- Binding crates (`phenotype-gfx-ffi`, `phenotype-gfx-wasm`) are thin; they have no
  algorithm logic and require minimal maintenance
