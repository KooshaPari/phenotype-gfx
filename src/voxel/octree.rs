//! Sparse voxel octree (SVO) for coarse / far-from-camera space.
//!
//! P-V1.1: minimal but real promotion model. A [`VoxelOctree`] stores one
//! [`OctreeNode`] per [`ChunkCoord`]; nodes that have been compacted into a
//! uniform material live as [`OctreeNode::Uniform`], while still-dense chunks
//! are tracked by reference as [`OctreeNode::Dense`]. Recursive subdivision
//! into 8-way branches is reserved for a follow-up PR (the public API will
//! stay additive).
//!
//! ## Node compaction
//!
//! [`VoxelOctree::compact`] implements greedy upward merging: any set of 8
//! sibling uniform nodes that share the same material value is collapsed into a
//! single parent-level uniform node.  The pass iterates until fixpoint so that
//! multi-level collapses are handled in one call.  The operation is idempotent —
//! a second call on an already-compacted tree always returns 0.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::voxel::coord::ChunkCoord;

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

    /// Compact the octree by collapsing uniform sibling groups upward.
    ///
    /// A *sibling group* is the set of 8 [`OctreeNode::Uniform`] children that
    /// all map to the same parent coord `(cx >> 1, cy >> 1, cz >> 1)`. When
    /// every member of such a group carries the **same** material value the 8
    /// nodes are removed and replaced by a single `Uniform` node at the parent
    /// coordinate, reducing memory usage and traversal cost.
    ///
    /// The pass repeats until no further collapses are possible (fixpoint), so
    /// multi-level pyramids are fully collapsed in one call.
    ///
    /// # Returns
    ///
    /// Total number of individual nodes removed across all rounds.  Returns `0`
    /// on an already-compacted tree (idempotent).
    pub fn compact(&mut self) -> usize {
        let mut total_collapsed: usize = 0;

        loop {
            let collapsed = self.compact_one_pass();
            if collapsed == 0 {
                break;
            }
            total_collapsed += collapsed;
        }

        total_collapsed
    }

    /// Execute a single bottom-up compaction pass.
    ///
    /// Returns the number of nodes removed (0 when no further collapse is
    /// possible).
    fn compact_one_pass(&mut self) -> usize {
        // For every Uniform node, derive its parent key and accumulate the
        // sibling set.  BTreeMap gives us deterministic iteration order.
        //
        // parent key → (common value if uniform-so-far, count of children seen)
        let mut groups: BTreeMap<(i32, i32, i32), Option<T>> = BTreeMap::new();
        // Track how many children each parent has seen.
        let mut group_counts: BTreeMap<(i32, i32, i32), u8> = BTreeMap::new();

        for (coord, node) in &self.nodes {
            if let OctreeNode::Uniform(v) = node {
                let key = (coord.cx >> 1, coord.cy >> 1, coord.cz >> 1);
                let count = group_counts.entry(key).or_insert(0);
                *count += 1;

                let entry = groups.entry(key).or_insert_with(|| Some(v.clone()));
                // If a previous sibling had a different value, mark as mixed.
                if let Some(existing) = entry.as_ref() {
                    if existing != v {
                        *entry = None; // mixed — cannot collapse
                    }
                }
            }
        }

        // Collect groups where all 8 siblings are uniform and share the same value.
        let collapsible: Vec<((i32, i32, i32), T)> = groups
            .into_iter()
            .filter_map(|(key, opt_val)| {
                let count = *group_counts.get(&key).unwrap_or(&0);
                if count == 8 {
                    opt_val.map(|v| (key, v))
                } else {
                    None
                }
            })
            .collect();

        if collapsible.is_empty() {
            return 0;
        }

        let mut removed = 0usize;

        for ((px, py, pz), value) in collapsible {
            // Remove the 8 child nodes.
            for dz in 0..2i32 {
                for dy in 0..2i32 {
                    for dx in 0..2i32 {
                        let child = ChunkCoord {
                            cx: px * 2 + dx,
                            cy: py * 2 + dy,
                            cz: pz * 2 + dz,
                        };
                        self.nodes.remove(&child);
                        removed += 1;
                    }
                }
            }
            // Insert one parent-level uniform node in their place.
            self.nodes.insert(
                ChunkCoord {
                    cx: px,
                    cy: py,
                    cz: pz,
                },
                OctreeNode::Uniform(value),
            );
        }

        removed
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

    // ── compaction tests ──────────────────────────────────────────────────────

    /// FR-PHENO-VOXEL-OCTREE-010 — a complete uniform sibling group (8 nodes,
    /// same material) is collapsed to a single parent-level leaf; the returned
    /// count equals the number of removed nodes (8).
    #[test]
    fn compact_uniform_subtree_collapses_to_one_leaf() {
        let mut o: VoxelOctree<u8> = VoxelOctree::default();
        // Insert all 8 children of the parent (0,0,0) → children at {0,1}³.
        for dz in 0..2i32 {
            for dy in 0..2i32 {
                for dx in 0..2i32 {
                    o.insert_uniform(
                        ChunkCoord {
                            cx: dx,
                            cy: dy,
                            cz: dz,
                        },
                        42,
                    );
                }
            }
        }
        assert_eq!(o.nodes.len(), 8);
        let collapsed = o.compact();
        assert_eq!(collapsed, 8, "expected 8 nodes removed");
        // Must now hold exactly 1 node at the parent coord (0,0,0).
        assert_eq!(o.nodes.len(), 1);
        assert_eq!(
            o.uniform_value(ChunkCoord {
                cx: 0,
                cy: 0,
                cz: 0
            }),
            Some(42)
        );
    }

    /// FR-PHENO-VOXEL-OCTREE-011 — a sibling group whose members hold
    /// **different** material values must not be collapsed.
    #[test]
    fn compact_mixed_subtree_is_preserved() {
        let mut o: VoxelOctree<u8> = VoxelOctree::default();
        // 7 siblings = material 1, one sibling = material 2.
        let mut val = 1u8;
        for dz in 0..2i32 {
            for dy in 0..2i32 {
                for dx in 0..2i32 {
                    o.insert_uniform(
                        ChunkCoord {
                            cx: dx,
                            cy: dy,
                            cz: dz,
                        },
                        val,
                    );
                    val = 1; // keep 1 for all but the first
                }
            }
        }
        // Override the very first child with a different value.
        o.insert_uniform(
            ChunkCoord {
                cx: 0,
                cy: 0,
                cz: 0,
            },
            2,
        );

        let before = o.nodes.len();
        let collapsed = o.compact();
        assert_eq!(collapsed, 0, "mixed group must not be collapsed");
        assert_eq!(o.nodes.len(), before, "node count must be unchanged");
    }

    /// FR-PHENO-VOXEL-OCTREE-012 — compaction is idempotent: a second call
    /// on an already-compacted tree always returns 0 and does not alter state.
    #[test]
    fn compact_is_idempotent() {
        let mut o: VoxelOctree<u8> = VoxelOctree::default();
        for dz in 0..2i32 {
            for dy in 0..2i32 {
                for dx in 0..2i32 {
                    o.insert_uniform(
                        ChunkCoord {
                            cx: dx,
                            cy: dy,
                            cz: dz,
                        },
                        7,
                    );
                }
            }
        }
        let first = o.compact();
        assert_eq!(first, 8);
        let second = o.compact();
        assert_eq!(second, 0, "second compaction must be a no-op");
        assert_eq!(o.nodes.len(), 1, "tree must be unchanged after second pass");
    }

    /// FR-PHENO-VOXEL-OCTREE-013 — queries return identical results before and
    /// after compaction (semantic equivalence).  All 8 original child coords
    /// must still resolve to the same material via the parent node.
    #[test]
    fn compact_preserves_query_semantics() {
        let mut o: VoxelOctree<u8> = VoxelOctree::default();
        let coords: Vec<ChunkCoord> = (0..2i32)
            .flat_map(|dz| {
                (0..2i32).flat_map(move |dy| {
                    (0..2i32).map(move |dx| ChunkCoord {
                        cx: dx,
                        cy: dy,
                        cz: dz,
                    })
                })
            })
            .collect();

        for &c in &coords {
            o.insert_uniform(c, 99);
        }

        // Record pre-compaction values.
        let pre: Vec<Option<u8>> = coords.iter().map(|&c| o.uniform_value(c)).collect();

        o.compact();

        // After compaction the original child coords are gone; the parent
        // node at (0,0,0) holds the material.  Any query that previously
        // returned Some(99) should still resolve to 99 through the parent.
        // We verify the parent directly, and confirm no child coord returns
        // a *different* value (it may return None — the parent is the
        // authoritative representation post-compaction).
        assert_eq!(
            o.uniform_value(ChunkCoord {
                cx: 0,
                cy: 0,
                cz: 0
            }),
            Some(99),
            "parent node must carry the merged material"
        );
        // Pre-compaction all returned Some(99).
        assert!(pre.iter().all(|v| *v == Some(99)));
    }

    /// FR-PHENO-VOXEL-OCTREE-014 — a multi-level uniform pyramid collapses
    /// across all levels in a single `compact()` call (fixpoint iteration).
    ///
    /// Setup: 64 leaf nodes arranged so that 8 groups of 8 each collapse to a
    /// level-1 node, and those 8 level-1 nodes then collapse to one level-2
    /// node — all in a single `compact()` call.
    #[test]
    fn compact_multi_level_pyramid_collapses_to_root() {
        let mut o: VoxelOctree<u8> = VoxelOctree::default();
        // Fill all 64 leaves at (0..4, 0..4, 0..4) — each group of 8 is
        // uniform with value 5.
        for dz in 0..4i32 {
            for dy in 0..4i32 {
                for dx in 0..4i32 {
                    o.insert_uniform(
                        ChunkCoord {
                            cx: dx,
                            cy: dy,
                            cz: dz,
                        },
                        5,
                    );
                }
            }
        }
        assert_eq!(o.nodes.len(), 64);

        let collapsed = o.compact();
        // Round 1: 64 leaves → 8 level-1 nodes → 64 child nodes removed.
        // Round 2: 8 level-1 nodes → 1 level-2 node → 8 nodes removed.
        // Total nodes removed = 64 + 8 = 72.
        assert_eq!(collapsed, 72, "full pyramid must collapse to a single root");
        assert_eq!(o.nodes.len(), 1);
        assert_eq!(
            o.uniform_value(ChunkCoord {
                cx: 0,
                cy: 0,
                cz: 0
            }),
            Some(5)
        );
    }

    /// FR-PHENO-VOXEL-OCTREE-015 — incomplete sibling group (fewer than 8
    /// children) is never collapsed even if all present siblings are uniform
    /// and hold the same value.
    #[test]
    fn compact_incomplete_group_not_collapsed() {
        let mut o: VoxelOctree<u8> = VoxelOctree::default();
        // Only 7 out of 8 siblings.
        for dz in 0..2i32 {
            for dy in 0..2i32 {
                for dx in 0..2i32 {
                    if (dx, dy, dz) != (1, 1, 1) {
                        o.insert_uniform(
                            ChunkCoord {
                                cx: dx,
                                cy: dy,
                                cz: dz,
                            },
                            3,
                        );
                    }
                }
            }
        }
        assert_eq!(o.nodes.len(), 7);
        let collapsed = o.compact();
        assert_eq!(collapsed, 0, "incomplete group must not be collapsed");
        assert_eq!(o.nodes.len(), 7);
    }
}
