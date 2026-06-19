//! Mesh adapters: concrete implementations of the [`Mesher`] port.
//!
//! These are re-exports of the engine-neutral meshers that live in the crate
//! root for backward compatibility. Future engine-specific adapters (Bevy, Godot,
//! Unreal) would be added here as additional implementations.

pub use crate::voxel::cubic_mesher::{CubicMesher, CubicVoxel};
pub use crate::voxel::greedy_mesher::GreedyMesher;

use crate::voxel::lod::LodLevel;
use crate::voxel::mesh::Mesher;
use crate::voxel::ports::chunk::ChunkView;
use crate::voxel::ports::mesh::{MeshBuffer, MeshResult};

/// Convenience type alias for the engine-neutral mesh buffer.
pub type NeutralMesh = MeshBuffer;

/// Convenience wrapper that implements [`Mesher`] with a concrete output type.
///
/// This is primarily useful for generic code that wants to dispatch to a
/// mesher without knowing whether it's cubic or greedy.
#[derive(Debug, Clone, Copy)]
pub enum MeshAdapter<V> {
    /// Cubic mesher (reference, exact, high triangle count).
    Cubic(CubicMesher<V>),
    /// Greedy mesher (merged quads, lower triangle count, AO-aware).
    Greedy(GreedyMesher<V>),
}

impl<V: CubicVoxel> Mesher for MeshAdapter<V> {
    type VoxelKind = V;
    type Mesh = MeshBuffer;

    fn mesh_chunk(&self, chunk: ChunkView<'_, V>, lod: LodLevel) -> MeshResult<Self::Mesh> {
        match self {
            MeshAdapter::Cubic(m) => m.mesh_chunk(chunk, lod),
            MeshAdapter::Greedy(m) => m.mesh_chunk(chunk, lod),
        }
    }
}
