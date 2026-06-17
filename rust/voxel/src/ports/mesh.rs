//! Mesh port: engine-neutral vertex / index buffers and the per-engine [`Mesher`] trait.

// Re-export domain types so the trait and implementations share the same types.
pub use crate::mesh::{MeshBuffer, MeshError, MeshResult, MeshVertex};

use crate::lod::LodLevel;
use crate::ports::chunk::ChunkView;

/// A per-engine adapter that turns a chunk view + LOD level into an engine-specific
/// mesh artifact (Bevy `Mesh`, Godot `ArrayMesh`, Unreal procedural mesh, …).
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
