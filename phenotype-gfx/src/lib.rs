//! `phenotype-gfx` — single-core graphics substrate.
//!
//! All algorithms live here exactly once. Thin FFI/SDK edges (cbindgen C-ABI for
//! C# P/Invoke, wasm-bindgen for TS/npm) are added in separate binding crates that
//! re-export from this core; they do not reimplement.
//!
//! See `docs/adr/0001-single-core-thin-ffi.md` for the locked architecture decision.

// Re-export the shared voxel kernel so callers import from one place.
pub use phenotype_voxel as kernel;
pub use phenotype_voxel::{
    select_lod, ChunkId, LodLevel, LodPolicy, MaterialId, VoxelScaleMultiplier,
};

pub mod lod;
pub mod postfx;
pub mod streaming;
pub mod voxel;
pub mod voxelizer;
pub mod water;
