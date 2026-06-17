//! phenotype-gfx: Single Rust core for unified graphics algorithms
//!
//! Holds all gfx algorithms (voxel, LOD, streaming, postfx, water, voxelizer) ONCE.
//! Thin FFI edges (C-ABI via cbindgen, wasm-bindgen) expose to consumers (C#, TS, web).
//! NO duplicated logic across languages.

pub mod voxel;
pub mod lod;
pub mod streaming;
pub mod postfx;
pub mod water;
pub mod voxelizer;
