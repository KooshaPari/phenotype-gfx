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

// OBSERVABILITY FAÇADE (L5 — 2026-06-30)
//
// Zero-cost when no tracing subscriber / metrics recorder is installed.
// Consumers opt in at their binary entry point; see `src/obs.rs` for the
// full metric catalogue and setup examples.
pub mod obs;

/// OTLP trace export and Prometheus metrics HTTP endpoint.
///
/// All functionality here is feature-gated behind `otlp` / `prometheus` /
/// `full-obs` features.  When the features are disabled this module is empty.
pub mod otlp;

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

/// Terrain system: height field, chunk mesh builder, terrain LOD, materials, shaders.
pub mod terrain;

/// Lighting system: SSAO, directional light, point light, shadow mapping.
pub mod lighting;

/// Compute shader framework for GPU-accelerated voxel processing.
pub mod compute;

/// Runtime plugin loading system: traits, context, and plugin manager.
pub mod plugin;

// Re-export the absorbed voxel kernel at the crate root so consumers that rename
// `phenotype-gfx` to `phenotype-voxel` keep the old `phenotype_voxel::Chunk` API.
pub use voxel as kernel;
pub use voxel::*;

// FFI EDGES (thin bindings, NOT logic) — feature-gated
/// C-ABI via cbindgen -> C# P/Invoke shim (WSM3D). Opaque handles wrap the Rust core.
#[cfg(feature = "c_api")]
#[path = "../bindings/c_api.rs"]
pub mod c_api;

// FUTURE: pub mod wasm;    // wasm-bindgen -> TS/npm (web)
