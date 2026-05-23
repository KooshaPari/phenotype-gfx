//! Sparse voxel octree (SVO) for coarse / far-from-camera space.
//!
//! P-V1.1: minimal but real promotion model. A [`VoxelOctree`] stores one
//! [`OctreeNode`] per [`ChunkCoord`]; nodes that have been compacted into a
//! uniform material live as [`OctreeNode::Uniform`], while still-dense chunks
//! are tracked by reference as [`OctreeNode::Dense`]. Recursive subdivision
//! into 8-way branches is reserved for a follow-up PR (the public API will
//! stay additive).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::coord::ChunkCoord;

/// One node in the sparse voxel octree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OctreeNode<T> {
    /// The entire chunk-volume holds a single value `T`. Cheap to store; reads
    /// return `T` without touching the dense chunk store.
    Uniform(T),
    /// The chunk is currently dense — see [`super::VoxelWorld::read`] which
    /// consults the dense chunk store. This variant only marks "I know about
    /// this coord and it has not been promoted yet."
    Dense,
}

/// Top-level handle on the world's sparse octree. Iteration is deterministic
/// (`BTreeMap` keyed on `ChunkCoord`); the public API never leaks
/// `HashMap`-style ordering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoxelOctree<T> {
    /// Nodes keyed by chunk-grid coordinate.
    pub nodes: BTreeMap<ChunkCoord, OctreeNode<T>>,
}

impl<T> Default for VoxelOctree<T> {
    fn default() -> Self {
        Self {
            nodes: BTreeMap::new(),
        }
    }
}

impl<T: Clone + PartialEq> VoxelOctree<T> {
    /// Insert a Uniform node for `coord` with value `value`. Overwrites any
    /// existing node at the same coord.
    pub fn insert_uniform(&mut self, coord: ChunkCoord, value: T) {
        self.nodes.insert(coord, OctreeNode::Uniform(value));
    }

    /// Mark `coord` as currently dense (no promotion possible / not yet
    /// performed).
    pub fn insert_dense(&mut self, coord: ChunkCoord) {
        self.nodes.insert(coord, OctreeNode::Dense);
    }

    /// Look up a chunk's node, if known.
    #[must_use]
    pub fn get(&self, coord: ChunkCoord) -> Option<&OctreeNode<T>> {
        self.nodes.get(&coord)
    }

    /// If `coord` is uniform, return the material. Otherwise `None`.
    #[must_use]
    pub fn uniform_value(&self, coord: ChunkCoord) -> Option<T> {
        match self.nodes.get(&coord) {
            Some(OctreeNode::Uniform(v)) => Some(v.clone()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// FR-PHENO-VOXEL-OCTREE-000 — empty octree has no nodes.
    #[test]
    fn empty_octree_has_no_nodes() {
        let o: VoxelOctree<u8> = VoxelOctree::default();
        assert!(o.nodes.is_empty());
    }

    /// FR-PHENO-VOXEL-OCTREE-001 — uniform insertion is recoverable.
    #[test]
    fn uniform_insert_roundtrips() {
        let mut o: VoxelOctree<u8> = VoxelOctree::default();
        let c = ChunkCoord {
            cx: 1,
            cy: 2,
            cz: 3,
        };
        o.insert_uniform(c, 42);
        assert_eq!(o.uniform_value(c), Some(42));
    }

    /// FR-PHENO-VOXEL-OCTREE-002 — dense markers do not appear as uniform.
    #[test]
    fn dense_marker_distinguished_from_uniform() {
        let mut o: VoxelOctree<u8> = VoxelOctree::default();
        let c = ChunkCoord {
            cx: 0,
            cy: 0,
            cz: 0,
        };
        o.insert_dense(c);
        assert!(o.uniform_value(c).is_none());
        assert!(matches!(o.get(c), Some(OctreeNode::Dense)));
    }
}
