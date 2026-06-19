//! phenotype-gfx: Single Rust core for unified graphics algorithms
//!
//! Holds all gfx algorithms (voxel, LOD, streaming, postfx, water, voxelizer,
//! terrain) ONCE. Thin FFI edges (C-ABI, wasm-bindgen) expose to consumers
//! (C#, TS, web). NO duplicated logic across languages.
//!
//! See `docs/adr/ADR-004-single-core-ffi-edges.md` for the locked architecture.
//!
//! ## Absorption history
//!
//! - L5-109 (2026-06-18): inlined `phenotype-voxel` into `voxel` (was: git dep).
//! - L5-110 (2026-06-18): ported C# `phenotype-terrain` into `terrain`.
//! - L5-111 (2026-06-18): ported C# `phenotype-water` into `water`.
//! - L5-112 (2026-06-18): ported C# + HLSL `phenotype-postfx` into `postfx`.

// ALGORITHM MODULES (all real logic lives here, exactly once)

/// Voxel kernel: storage, meshing, chunk management; PBR material policy.
pub mod voxel;

/// LOD system: frustum culling, chunk render planning, scale-budget primitives.
pub mod lod;

/// Streaming window policy: ring-based chunk lifecycle, eviction ordering.
pub mod streaming;

/// Post-processing pipeline: SSAO, SSGI, Bloom, ACES, LUT, vignette, CA.
pub mod postfx;

/// Water simulation: Gerstner waves, fluid mesh generation, water LOD.
pub mod water;

/// Sprite voxelizer: voxel-to-sprite rendering (OrganicBlob, Lathe, PerTexel).
pub mod voxelizer;

/// Terrain system: height field, chunk mesh builder, terrain LOD, materials.
pub mod terrain;

// FUTURE: FFI EDGES (thin bindings, NOT logic)
// pub mod c_api;   // C-ABI via cbindgen → C# P/Invoke shim (WSM3D)
// pub mod wasm;    // wasm-bindgen → TS/npm (web)
