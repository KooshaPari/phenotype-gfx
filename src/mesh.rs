//! Mesh-neutral vertex / index buffers and the per-engine [`Mesher`] trait.
//!
//! The substrate ships mesh-neutral buffers; each renderer (Bevy / Godot / Unreal)
//! supplies its own implementation in its client crate.

use serde::{Deserialize, Serialize};

use crate::chunk::ChunkView;
use crate::lod::LodLevel;
use crate::material::MaterialId;

/// Engine-neutral vertex layout. PBR-suitable: position + normal + uv + material slot.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MeshVertex {
    /// Position in world space (already converted out of fixed-point at the renderer
    /// boundary by the caller — meshers see `f32` for vertex math).
    pub position: [f32; 3],
    /// Surface normal.
    pub normal: [f32; 3],
    /// UV (planar projection by default).
    pub uv: [f32; 2],
    /// Material slot. Renderer translates to its own PBR material set.
    pub material: MaterialId,
}

/// Mesh-neutral indexed-triangle buffer produced by a [`Mesher`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MeshBuffer {
    /// Vertex array.
    pub vertices: Vec<MeshVertex>,
    /// Triangle indices. Length must be a multiple of 3.
    pub indices: Vec<u32>,
    /// Per-vertex ambient occlusion values, parallel to `vertices`.
    ///
    /// Each entry is in `0..=3`: **3 = fully lit** (no occlusion), **0 = maximum
    /// occlusion** (vertex is wedged into a solid corner).  The classic voxel-AO
    /// rule: if both side neighbours are solid the value is 0, otherwise
    /// `3 - (side1_solid + side2_solid + corner_solid)`.
    ///
    /// `CubicMesher` populates this field.  `GreedyMesher` leaves all entries at
    /// the default of 3 (TODO: add greedy AO in a future pass).
    /// Length is always equal to `vertices.len()`.
    pub ao: Vec<u8>,
}

/// Result of a mesher pass.
pub type MeshResult<T> = Result<T, MeshError>;

/// Mesher error type. Renderers can extend this in their own crates via wrappers.
#[derive(Debug, thiserror::Error)]
pub enum MeshError {
    /// The chunk view did not contain the expected number of voxels.
    #[error("chunk view has unexpected length: got {got}, expected {expected}")]
    BadChunkSize {
        /// Actual length received.
        got: usize,
        /// Length the mesher expected.
        expected: usize,
    },
}

/// A per-engine adapter that turns a chunk view + LOD level into an engine-specific
/// mesh artifact (Bevy `Mesh`, Godot `ArrayMesh`, Unreal procedural mesh, …).
///
/// The associated `VoxelKind` type pins which voxel value type this mesher
/// consumes, eliminating the unsound `mesh_chunk<T>` generic that previously
/// couldn't enforce `T: CubicVoxel` at the trait boundary.
pub trait Mesher {
    /// Voxel value type this mesher consumes.
    type VoxelKind: Default + Clone;
    /// Engine-specific mesh artifact type.
    type Mesh;
    /// Mesh `chunk` at level `lod`. Implementations should be deterministic for a
    /// given (chunk, lod) pair so replay produces identical meshes.
    fn mesh_chunk(
        &self,
        chunk: ChunkView<'_, Self::VoxelKind>,
        lod: LodLevel,
    ) -> MeshResult<Self::Mesh>;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// FR-PHENO-VOXEL-MESH-000 — `MeshBuffer::default` is empty and serializable.
    #[test]
    fn default_meshbuffer_is_empty() {
        let m = MeshBuffer::default();
        assert!(m.vertices.is_empty());
        assert!(m.indices.is_empty());
    }
}
