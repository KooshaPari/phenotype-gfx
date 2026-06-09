//! Trait-driven ports (contracts) for the voxel substrate.
//!
//! Every adapter — mesher, renderer, serializer — implements one of these traits.
//! The domain code depends **only** on these traits, never on concrete adapters.

pub mod chunk;
pub mod mesh;
pub mod octree;

pub use chunk::{ChunkId, ChunkView, Chunkable};
pub use mesh::{MeshBuffer, MeshError, MeshResult, MeshVertex, Mesher};
pub use octree::{OctreeQueryable, OctreeStorage};
