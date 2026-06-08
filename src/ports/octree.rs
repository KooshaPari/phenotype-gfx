//! Octree port: sparse voxel tree query and storage contracts.

use crate::coord::ChunkCoord;

// Re-export the domain type so the trait and implementation share the same type.
pub use crate::octree::OctreeNode;

/// Query contract: uniform-value lookup and node inspection.
pub trait OctreeQueryable<T: Clone> {
    /// Look up a chunk's node, if known.
    fn get(&self, coord: ChunkCoord) -> Option<&OctreeNode<T>>;
    /// If `coord` is uniform, return the material. Otherwise `None`.
    fn uniform_value(&self, coord: ChunkCoord) -> Option<T>;
    /// Compact the octree by collapsing uniform sibling groups upward.
    fn compact(&mut self) -> usize;
}

/// Storage contract: insert and modify nodes.
pub trait OctreeStorage<T: Clone + PartialEq>: OctreeQueryable<T> {
    /// Insert a Uniform node for `coord` with value `value`. Overwrites any
    /// existing node at the same coord.
    fn insert_uniform(&mut self, coord: ChunkCoord, value: T);
    /// Mark `coord` as currently dense.
    fn insert_dense(&mut self, coord: ChunkCoord);
}
