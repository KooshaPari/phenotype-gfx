//! Sparse voxel octree (SVO) for coarse / far-from-camera space.
//!
//! This module ships a minimal, deterministic SVO skeleton. The first real
//! implementation lands in P-V1 (Civis) / first-real-PR (phenotype-voxel) — for now
//! it carries the public types so consumers can wire against the API.

use serde::{Deserialize, Serialize};

use crate::chunk::ChunkId;

/// One node in a sparse voxel octree. Children may be absent when the subtree is
/// uniform.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OctreeNode {
    /// The eight child IDs in canonical Morton order. `None` means the subtree is
    /// uniform (the parent's material value applies to the whole child volume).
    pub children: [Option<ChunkId>; 8],
}

impl OctreeNode {
    /// A uniform node with no children.
    #[must_use]
    pub const fn uniform() -> Self {
        Self {
            children: [None; 8],
        }
    }
}

/// Top-level handle on the world's sparse octree.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct VoxelOctree {
    /// Nodes keyed by their [`ChunkId`]. `BTreeMap` would land here in the real
    /// implementation to preserve deterministic iteration; the skeleton uses
    /// `Vec<(ChunkId, OctreeNode)>` for now (sorted on access).
    pub nodes: Vec<(ChunkId, OctreeNode)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// FR-PHENO-VOXEL-OCTREE-000 — `OctreeNode::uniform` is empty.
    #[test]
    fn uniform_node_has_no_children() {
        let n = OctreeNode::uniform();
        assert!(n.children.iter().all(Option::is_none));
    }
}
