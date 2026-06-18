//! Chunk storage: dense 16³ leaf chunks indexed by [`ChunkId`].

use serde::{Deserialize, Serialize};

/// Dense leaf chunks are always 16 voxels on a side; total `16^3 = 4096` voxels.
pub const CHUNK_EDGE: usize = 16;
/// Total voxels in a dense leaf chunk.
pub const CHUNK_VOXELS: usize = CHUNK_EDGE * CHUNK_EDGE * CHUNK_EDGE;

/// Stable, hashable identifier for a chunk in the world. Encodes chunk-grid
/// coordinates as a single `u64` so it can be used as a deterministic key without
/// committing to a particular `HashMap` iteration order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ChunkId(pub u64);

/// Owned dense leaf chunk parameterised over the voxel value type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk<T: Default + Clone> {
    /// Voxel storage in `x + y * EDGE + z * EDGE * EDGE` order.
    pub voxels: Vec<T>,
}

impl<T: Default + Clone> Default for Chunk<T> {
    fn default() -> Self {
        Self {
            voxels: vec![T::default(); CHUNK_VOXELS],
        }
    }
}

/// Borrowed view of a chunk plus its bounds; given to meshers so they can produce
/// engine-specific mesh buffers without taking ownership of voxel storage.
#[derive(Debug, Clone, Copy)]
pub struct ChunkView<'a, T> {
    /// Stable ID of the chunk this view describes.
    pub id: ChunkId,
    /// Borrowed slice of voxel values, length [`CHUNK_VOXELS`].
    pub voxels: &'a [T],
}

#[cfg(test)]
mod tests {
    use super::*;

    /// FR-PHENO-VOXEL-CHUNK-000 — chunk constants are consistent.
    #[test]
    fn chunk_constants_are_consistent() {
        assert_eq!(CHUNK_VOXELS, CHUNK_EDGE * CHUNK_EDGE * CHUNK_EDGE);
    }

    /// FR-PHENO-VOXEL-CHUNK-001 — default chunk has the expected voxel count.
    #[test]
    fn default_chunk_has_expected_voxel_count() {
        let c: Chunk<u8> = Chunk::default();
        assert_eq!(c.voxels.len(), CHUNK_VOXELS);
    }
}
