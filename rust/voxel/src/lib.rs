//! # phenotype-voxel
//!
//! Adaptive voxel substrate for Phenotype-org games.
//!
//! Primary representation: **sparse voxel octree (SVO) for coarse / far-from-camera
//! space + dense 16³ leaf chunks for near-camera detail**. Every write produces a
//! deterministic [`DirtyChunkEvent`] so consumers (Civis, WorldSphereMod3D) can
//! rebuild meshes in a replay-safe order.
//!
//! ## Design references
//!
//! - Civis 3D extension plan: see the Civis repo `docs/roadmap/civis-3d-extension.md`.
//! - Adaptive voxel ADR: see Civis `docs/adr/ADR-005-adaptive-voxel.md`.
//!
//! ## Determinism contract
//!
//! - World coordinates are fixed-point `i64` at `10^6` scale. No `f32`/`f64` crosses
//!   the public API.
//! - Dirty events are ordered by `(chunk_id, write_seq)`. Iteration of internal
//!   collections never leaks ordering into the public surface.
//! - `VoxelScaleMultiplier` is a first-class semantic with a sensible default; LOD
//!   selection composes with it through provided helpers so consumers cannot
//!   accidentally desynchronise (WSM3D-lineage invariant).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod adapters;
#[cfg(feature = "bevy")]
pub mod bevy_adapter;
pub mod chunk;
pub mod coord;
pub mod cubic_mesher;
pub mod delta;
pub mod fixtures;
pub mod greedy_mesher;
pub mod lod;
pub mod material;
pub mod mesh;
pub mod octree;
pub mod ports;
pub mod serial;
pub mod shape_hints;
pub mod sprite_voxelizer;
pub mod world;

pub use adapters::{DenseChunkStore, MeshAdapter, OctreeAdapter, VoxelWorldAdapter};
#[cfg(feature = "bevy")]
pub use bevy_adapter::to_bevy_mesh;
pub use chunk::{Chunk, ChunkId, ChunkView};
pub use coord::{to_chunk_coord, ChunkCoord, WorldCoord, FIXED_SCALE};
pub use cubic_mesher::{CubicMesher, CubicVoxel};
pub use delta::{DirtyChunkEvent, WriteSeq};
pub use greedy_mesher::GreedyMesher;
pub use lod::{select_lod, LodLevel, LodPolicy, VoxelScaleMultiplier};
pub use material::{MaterialId, MaterialPalette, VoxelMaterial};
pub use mesh::{MeshBuffer, MeshError, MeshResult, MeshVertex, Mesher};
pub use octree::{OctreeNode, VoxelOctree};
pub use ports::{
    Camera, Chunkable, FrameId, OctreeQueryable, OctreeStorage, RenderError, RenderResult,
    RendererPort, WorldStore,
};
pub use serial::{load_chunk, save_chunk};
pub use shape_hints::{ShapeHint, ShapeHintRegistry};
pub use sprite_voxelizer::{
    compute_manhattan_dist_to_air, voxelize_image, voxelize_to_chunk, ExtrusionMode, SpriteVoxel,
    VoxelizeConfig, DEFAULT_DEPTH,
};
pub use world::VoxelWorld;

/// Schema version of the public `phenotype-voxel` types. Bumped on breaking changes
/// so consumers can detect API drift in `.civreplay` / serialized voxel diffs.
pub const SCHEMA_VERSION: u32 = 1;

/// Default `VoxelScaleMultiplier` derived from WSM3D's hard-won visible-default lesson
/// (mesh-local 11x5x1 ✕ sprite-scale 0.1 ≈ ~1.1x0.5x0.1 world → invisible; multiplier
/// of 8 brought it back to a usable rendered scale).
pub const DEFAULT_VOXEL_SCALE_MULTIPLIER: f32 = 8.0;

/// Return whether a persisted wire format schema version is supported by this
/// crate revision. This avoids version checks being repeated across downstream
/// bridges.
#[inline]
pub const fn is_supported_schema_version(version: u32) -> bool {
    version == SCHEMA_VERSION
}
