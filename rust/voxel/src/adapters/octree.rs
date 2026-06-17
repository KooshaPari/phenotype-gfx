//! Octree adapter: implements the [`OctreeQueryable`] and [`OctreeStorage`] ports
//! on the domain [`VoxelOctree`] type.

use crate::coord::ChunkCoord;
use crate::octree::{OctreeNode, VoxelOctree};
use crate::ports::octree::{OctreeQueryable, OctreeStorage};

/// Thin adapter that wraps [`VoxelOctree`] and implements the port traits.
///
/// This is a zero-cost wrapper — it just forwards to the underlying octree
/// methods. In a future refactor `VoxelOctree` could implement the traits
/// directly, eliminating this wrapper.
#[derive(Debug, Clone, Default)]
pub struct OctreeAdapter<T: Clone + PartialEq> {
    inner: VoxelOctree<T>,
}

impl<T: Clone + PartialEq> OctreeAdapter<T> {
    /// Construct from an existing [`VoxelOctree`].
    pub fn new(inner: VoxelOctree<T>) -> Self {
        Self { inner }
    }

    /// Consume the adapter and return the underlying [`VoxelOctree`].
    pub fn into_inner(self) -> VoxelOctree<T> {
        self.inner
    }

    /// Borrow the underlying [`VoxelOctree`].
    pub fn inner(&self) -> &VoxelOctree<T> {
        &self.inner
    }

    /// Mutably borrow the underlying [`VoxelOctree`].
    pub fn inner_mut(&mut self) -> &mut VoxelOctree<T> {
        &mut self.inner
    }

    /// Compact the octree and return the number of nodes removed.
    pub fn compact(&mut self) -> usize {
        self.inner.compact()
    }

    /// Insert a uniform node.
    pub fn insert_uniform(&mut self, coord: ChunkCoord, value: T) {
        self.inner.insert_uniform(coord, value);
    }

    /// Insert a dense marker.
    pub fn insert_dense(&mut self, coord: ChunkCoord) {
        self.inner.insert_dense(coord);
    }

    /// Get a node reference.
    pub fn get(&self, coord: ChunkCoord) -> Option<&OctreeNode<T>> {
        self.inner.get(coord)
    }

    /// Get a uniform value if the node is uniform.
    pub fn uniform_value(&self, coord: ChunkCoord) -> Option<T> {
        self.inner.uniform_value(coord)
    }

    /// Number of nodes in the octree.
    pub fn len(&self) -> usize {
        self.inner.nodes.len()
    }

    /// True when the octree has no nodes.
    pub fn is_empty(&self) -> bool {
        self.inner.nodes.is_empty()
    }
}

impl<T: Clone + PartialEq> OctreeQueryable<T> for OctreeAdapter<T> {
    fn get(&self, coord: ChunkCoord) -> Option<&OctreeNode<T>> {
        self.inner.get(coord)
    }

    fn uniform_value(&self, coord: ChunkCoord) -> Option<T> {
        self.inner.uniform_value(coord)
    }

    fn compact(&mut self) -> usize {
        self.inner.compact()
    }
}

impl<T: Clone + PartialEq> OctreeStorage<T> for OctreeAdapter<T> {
    fn insert_uniform(&mut self, coord: ChunkCoord, value: T) {
        self.inner.insert_uniform(coord, value);
    }

    fn insert_dense(&mut self, coord: ChunkCoord) {
        self.inner.insert_dense(coord);
    }
}
