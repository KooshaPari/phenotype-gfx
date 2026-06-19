//! phenotype-gfx: Single Rust core for unified graphics algorithms
//!
//! Holds all gfx algorithms (voxel, LOD, streaming, postfx, water, voxelizer) ONCE.
//! Thin FFI edges (C-ABI, wasm-bindgen) expose to consumers (C#, TS, web).
//! NO duplicated logic across languages.
//!
//! See `docs/adr/ADR-004-single-core-ffi-edges.md` for the locked architecture.

// Re-export the shared voxel kernel so consumers import from one place.
pub use phenotype_voxel as kernel;
pub use phenotype_voxel::*;

// ALGORITHM MODULES (all real logic lives here, exactly once)

/// Voxel kernel: storage, meshing, chunk management; PBR material policy.
pub mod voxel;

/// LOD system: frustum culling, chunk render planning, scale-budget primitives.
pub mod lod;

/// Streaming window policy: ring-based chunk lifecycle, eviction ordering.
pub mod streaming;

/// Post-processing pipeline: SSAO, SSGI, Bloom, ACES, LUT.
pub mod postfx;

/// Water simulation: Gerstner waves, fluid mesh generation.
pub mod water;

/// Sprite voxelizer: voxel-to-sprite rendering (OrganicBlob, Lathe, PerTexel).
pub mod voxelizer;

// FUTURE: FFI EDGES (thin bindings, NOT logic)
// pub mod c_api;   // C-ABI via cbindgen → C# P/Invoke shim (WSM3D)
// pub mod wasm;    // wasm-bindgen → TS/npm (web)
