//! Chunk port: identity and view contracts.

// Re-export domain types so the trait and implementations share the same types.
pub use crate::voxel::chunk::{ChunkId, ChunkView, CHUNK_EDGE, CHUNK_VOXELS};

/// Trait for types that can be viewed as a chunk of voxels.
pub trait Chunkable {
    /// The voxel value type.
    type Voxel: Default + Clone;
    /// Return a view of the chunk at the given ID, if it exists.
    fn view(&self, id: ChunkId) -> Option<ChunkView<'_, Self::Voxel>>;
}
