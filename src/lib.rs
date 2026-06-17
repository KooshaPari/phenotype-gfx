//! phenotype-gfx: Single Rust core for unified graphics algorithms
//!
//! Holds all gfx algorithms (voxel, LOD, streaming, postfx, water, voxelizer) ONCE.
//! Thin FFI edges (C-ABI, wasm-bindgen) expose to consumers (C#, TS, web).
//! NO duplicated logic across languages.
//!
//! See `docs/adr/ADR-004-single-core-ffi-edges.md` and
//! `docs/adr/0001-single-core-thin-ffi.md` for the locked architecture.

// SHARED KERNEL RE-EXPORTS (one import home for consumers)
pub use phenotype_voxel as kernel;
pub use phenotype_voxel::{
    select_lod, ChunkId, LodLevel, LodPolicy, MaterialId, VoxelScaleMultiplier,
};

// ALGORITHM MODULES (all real logic lives here)

/// Voxel kernel: storage, meshing, chunk management
pub mod voxel;

/// LOD system: frustum culling, chunk render planning
pub mod lod;

/// Streaming: chunk window, ring-based LOD, eviction
pub mod streaming;

/// Post-processing pipeline: SSAO, SSGI, Bloom, ACES, LUT
pub mod postfx;

/// Water simulation: Gerstner waves, fluid mesh generation
pub mod water;

/// Sprite voxelizer: voxel-to-sprite rendering (OrganicBlob, Lathe, PerTexel)
pub mod voxelizer;

// FUTURE: FFI EDGES (thin bindings, NOT logic)
// pub mod c_api;   // C-ABI via cbindgen (future)
