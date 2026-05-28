//! Top-level `VoxelWorld`: deterministic write + drain over the hybrid SVO + dense
//! leaf-chunk storage.
//!
//! P-V1 milestone: only dense-leaf writes are implemented end-to-end; SVO-level
//! sparse upgrades land in a follow-up PR. The public API is shaped so that future
//! sparse upgrades are additive (callers always go through [`VoxelWorld::write`]
//! and [`VoxelWorld::drain_dirty`]).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::chunk::{Chunk, CHUNK_EDGE, CHUNK_VOXELS};
use crate::coord::{to_chunk_coord, ChunkCoord, WorldCoord};
use crate::delta::{DirtyChunkEvent, WriteSeq};
use crate::octree::VoxelOctree;

/// World container.
///
/// Storage is keyed by [`ChunkCoord`] in a [`BTreeMap`] so iteration order is
/// deterministic (replay-safe by construction; see ADR-004 in the Civis repo).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoxelWorld<T: Default + Clone + PartialEq> {
    /// Fixed-point world-units that one voxel edge spans.
    voxel_span: i64,
    /// Dense leaf chunks indexed by chunk-grid coordinates.
    chunks: BTreeMap<ChunkCoord, Chunk<T>>,
    /// Sparse voxel octree for coordinates whose dense leaves have been
    /// promoted into uniform values. Reads fall through to this when a
    /// `ChunkCoord` is absent from `chunks` (see [`VoxelWorld::read`]).
    octree: VoxelOctree<T>,
    /// Monotonic write sequence.
    write_seq: WriteSeq,
    /// Dirty events not yet drained. Always kept sorted by `(chunk_id, write_seq)`
    /// on `drain_dirty` so callers do not need to sort defensively.
    dirty: Vec<DirtyChunkEvent>,
}

impl<T: Default + Clone + PartialEq> VoxelWorld<T> {
    /// Construct an empty world. `voxel_span` is how many fixed-point world units
    /// one voxel edge spans (e.g. `FIXED_SCALE` for a 1m voxel at the default
    /// scale).
    #[must_use]
    pub fn new(voxel_span: i64) -> Self {
        Self {
            voxel_span,
            chunks: BTreeMap::new(),
            octree: VoxelOctree::default(),
            write_seq: WriteSeq::default(),
            dirty: Vec::new(),
        }
    }

    /// Write a voxel at the given world position. Lazily allocates the containing
    /// chunk on first write. Returns the chunk-grid coordinate that was touched.
    ///
    /// If the write does not actually change the stored value, no dirty event is
    /// emitted (idempotent writes do not pollute the replay log).
    pub fn write(&mut self, pos: WorldCoord, value: T) -> ChunkCoord {
        let coord = to_chunk_coord(pos, self.voxel_span, CHUNK_EDGE as i32);
        let chunk = self.chunks.entry(coord).or_insert_with(|| Chunk {
            voxels: vec![T::default(); CHUNK_VOXELS],
        });

        let edge = CHUNK_EDGE as i64;
        let world_edge = self.voxel_span * edge;
        let local_x = pos.x.rem_euclid(world_edge).div_euclid(self.voxel_span) as usize;
        let local_y = pos.y.rem_euclid(world_edge).div_euclid(self.voxel_span) as usize;
        let local_z = pos.z.rem_euclid(world_edge).div_euclid(self.voxel_span) as usize;
        let idx = local_x + local_y * CHUNK_EDGE + local_z * CHUNK_EDGE * CHUNK_EDGE;

        if chunk.voxels[idx] != value {
            chunk.voxels[idx] = value;
            self.dirty.push(DirtyChunkEvent {
                chunk_id: coord.chunk_id(),
                write_seq: self.write_seq.advance(),
            });
        }

        coord
    }

    /// Drain all pending dirty events in `(chunk_id, write_seq)` order.
    ///
    /// Replay-safety: the returned vector is always sorted so that consumers can
    /// rely on iteration order being identical across machines and replays.
    pub fn drain_dirty(&mut self) -> Vec<DirtyChunkEvent> {
        let mut out = std::mem::take(&mut self.dirty);
        out.sort();
        out
    }

    /// Read a voxel at the given world position. Falls back to the sparse
    /// octree when the containing chunk has been compacted into a uniform
    /// region. Returns the default value when neither store knows about the
    /// coord.
    #[must_use]
    pub fn read(&self, pos: WorldCoord) -> T {
        let coord = to_chunk_coord(pos, self.voxel_span, CHUNK_EDGE as i32);
        if let Some(chunk) = self.chunks.get(&coord) {
            let edge = CHUNK_EDGE as i64;
            let world_edge = self.voxel_span * edge;
            let local_x = pos.x.rem_euclid(world_edge).div_euclid(self.voxel_span) as usize;
            let local_y = pos.y.rem_euclid(world_edge).div_euclid(self.voxel_span) as usize;
            let local_z = pos.z.rem_euclid(world_edge).div_euclid(self.voxel_span) as usize;
            let idx = local_x + local_y * CHUNK_EDGE + local_z * CHUNK_EDGE * CHUNK_EDGE;
            return chunk.voxels[idx].clone();
        }
        if let Some(v) = self.octree.uniform_value(coord) {
            return v;
        }
        T::default()
    }

    /// Number of allocated *dense* chunks. Test-friendly observability.
    #[must_use]
    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    /// Total voxels represented by the dense chunk store.
    #[must_use]
    pub fn total_voxel_count(&self) -> usize {
        self.chunks.len() * CHUNK_VOXELS
    }

    /// Borrow a dense chunk by chunk-grid coordinate.
    ///
    /// Returns `None` when the chunk has been compacted into the sparse octree;
    /// callers should use [`VoxelWorld::octree`] / [`VoxelOctree::uniform_value`]
    /// for uniform regions.
    #[must_use]
    pub fn chunk(&self, coord: ChunkCoord) -> Option<&Chunk<T>> {
        self.chunks.get(&coord)
    }

    /// Iterate over all dense chunks in deterministic `BTreeMap` order.
    pub fn chunks_dense(&self) -> impl Iterator<Item = (ChunkCoord, &Chunk<T>)> {
        self.chunks.iter().map(|(coord, chunk)| (*coord, chunk))
    }

    /// Return the dense chunks named by `coords` in the order provided.
    pub fn chunks_dense_at(&self, coords: &[ChunkCoord]) -> Vec<(ChunkCoord, &Chunk<T>)> {
        coords
            .iter()
            .filter_map(|coord| {
                self.chunks
                    .get_key_value(coord)
                    .map(|(c, chunk)| (*c, chunk))
            })
            .collect()
    }

    /// Number of chunks promoted into the sparse octree as uniform regions.
    #[must_use]
    pub fn uniform_chunk_count(&self) -> usize {
        self.octree.nodes.len()
    }

    /// Borrow the sparse octree (read-only).
    #[must_use]
    pub fn octree(&self) -> &VoxelOctree<T> {
        &self.octree
    }

    /// Compact the world: walk dense chunks, identify those that are fully
    /// uniform, drop them from dense storage, and promote them into the sparse
    /// octree as [`crate::octree::OctreeNode::Uniform`].
    ///
    /// Returns the number of chunks promoted in this pass. The operation is
    /// idempotent — re-running on an already-compacted world is a no-op.
    pub fn compact(&mut self) -> usize {
        let mut promote: Vec<(ChunkCoord, T)> = Vec::new();
        for (coord, chunk) in &self.chunks {
            if let Some(first) = chunk.voxels.first() {
                let uniform = chunk.voxels.iter().all(|v| v == first);
                if uniform {
                    promote.push((*coord, first.clone()));
                }
            }
        }
        for (coord, value) in &promote {
            self.chunks.remove(coord);
            self.octree.insert_uniform(*coord, value.clone());
        }
        promote.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// FR-PHENO-VOXEL-WORLD-001 — empty world is empty; reads return default.
    #[test]
    fn empty_world_reads_default() {
        let w: VoxelWorld<u8> = VoxelWorld::new(1_000_000);
        let v = w.read(WorldCoord { x: 0, y: 0, z: 0 });
        assert_eq!(v, 0);
        assert_eq!(w.chunk_count(), 0);
    }

    /// FR-PHENO-VOXEL-WORLD-002 — write allocates the containing chunk and emits a
    /// dirty event; read returns the written value.
    #[test]
    fn write_then_read_roundtrips() {
        let mut w: VoxelWorld<u8> = VoxelWorld::new(1_000_000);
        let pos = WorldCoord {
            x: 5_000_000,
            y: 0,
            z: 0,
        };
        w.write(pos, 42);
        assert_eq!(w.read(pos), 42);
        assert_eq!(w.chunk_count(), 1);
        let dirty = w.drain_dirty();
        assert_eq!(dirty.len(), 1);
    }

    /// FR-PHENO-VOXEL-WORLD-003 — idempotent writes do not produce dirty events.
    #[test]
    fn idempotent_write_emits_no_event() {
        let mut w: VoxelWorld<u8> = VoxelWorld::new(1_000_000);
        let pos = WorldCoord { x: 0, y: 0, z: 0 };
        w.write(pos, 7);
        let _ = w.drain_dirty();
        w.write(pos, 7);
        assert!(w.drain_dirty().is_empty());
    }

    /// FR-PHENO-VOXEL-WORLD-004 — dirty events drain in `(chunk_id, write_seq)`
    /// order regardless of write interleaving.
    #[test]
    fn dirty_events_drain_in_sorted_order() {
        let mut w: VoxelWorld<u8> = VoxelWorld::new(1_000_000);
        // Two distinct chunks, two writes each, interleaved.
        let a0 = WorldCoord { x: 0, y: 0, z: 0 };
        let b0 = WorldCoord {
            x: 100_000_000,
            y: 0,
            z: 0,
        };
        let a1 = WorldCoord {
            x: 1_000_000,
            y: 0,
            z: 0,
        };
        let b1 = WorldCoord {
            x: 101_000_000,
            y: 0,
            z: 0,
        };
        w.write(a0, 1);
        w.write(b0, 1);
        w.write(a1, 1);
        w.write(b1, 1);
        let dirty = w.drain_dirty();
        assert_eq!(dirty.len(), 4);
        // Within each chunk, write_seq must be ascending.
        for window in dirty.windows(2) {
            if window[0].chunk_id == window[1].chunk_id {
                assert!(window[0].write_seq <= window[1].write_seq);
            } else {
                assert!(window[0].chunk_id < window[1].chunk_id);
            }
        }
    }

    /// FR-PHENO-VOXEL-WORLD-006 — `compact()` promotes a uniform dense chunk
    /// into the sparse octree, drops it from dense storage, and read() still
    /// returns the correct material via the octree fallback.
    #[test]
    fn compact_promotes_uniform_chunks() {
        let mut w: VoxelWorld<u8> = VoxelWorld::new(1_000_000);
        // Fill a 16-cell column at the origin so the whole 16³ chunk = 1.
        for z in 0..16 {
            for y in 0..16 {
                for x in 0..16 {
                    w.write(
                        WorldCoord {
                            x: x * 1_000_000,
                            y: y * 1_000_000,
                            z: z * 1_000_000,
                        },
                        1u8,
                    );
                }
            }
        }
        assert_eq!(w.chunk_count(), 1);
        let promoted = w.compact();
        assert_eq!(promoted, 1);
        assert_eq!(w.chunk_count(), 0);
        assert_eq!(w.uniform_chunk_count(), 1);
        // Read still works through the octree fallback.
        assert_eq!(w.read(WorldCoord { x: 0, y: 0, z: 0 }), 1);
        assert_eq!(
            w.read(WorldCoord {
                x: 5_000_000,
                y: 5_000_000,
                z: 5_000_000
            }),
            1
        );
    }

    /// FR-PHENO-VOXEL-WORLD-007 — `compact()` does not promote a chunk that
    /// has any non-uniform voxels.
    #[test]
    fn compact_skips_non_uniform_chunks() {
        let mut w: VoxelWorld<u8> = VoxelWorld::new(1_000_000);
        w.write(WorldCoord { x: 0, y: 0, z: 0 }, 1);
        w.write(
            WorldCoord {
                x: 1_000_000,
                y: 0,
                z: 0,
            },
            2,
        );
        let promoted = w.compact();
        assert_eq!(promoted, 0);
        assert_eq!(w.chunk_count(), 1);
        assert_eq!(w.uniform_chunk_count(), 0);
    }

    /// FR-PHENO-VOXEL-WORLD-008 — repeated `compact()` is idempotent.
    #[test]
    fn compact_is_idempotent() {
        let mut w: VoxelWorld<u8> = VoxelWorld::new(1_000_000);
        for z in 0..16 {
            for y in 0..16 {
                for x in 0..16 {
                    w.write(
                        WorldCoord {
                            x: x * 1_000_000,
                            y: y * 1_000_000,
                            z: z * 1_000_000,
                        },
                        7,
                    );
                }
            }
        }
        let first = w.compact();
        let second = w.compact();
        assert_eq!(first, 1);
        assert_eq!(second, 0);
        assert_eq!(w.uniform_chunk_count(), 1);
    }

    /// FR-PHENO-VOXEL-WORLD-009 — replay determinism holds across compact()
    /// boundaries: two worlds that follow identical write+compact sequences
    /// share identical state.
    #[test]
    fn replay_holds_across_compact() {
        fn build(w: &mut VoxelWorld<u8>) {
            for z in 0..16 {
                for y in 0..16 {
                    for x in 0..16 {
                        w.write(
                            WorldCoord {
                                x: x * 1_000_000,
                                y: y * 1_000_000,
                                z: z * 1_000_000,
                            },
                            9,
                        );
                    }
                }
            }
        }
        let mut w1: VoxelWorld<u8> = VoxelWorld::new(1_000_000);
        let mut w2: VoxelWorld<u8> = VoxelWorld::new(1_000_000);
        build(&mut w1);
        build(&mut w2);
        assert_eq!(w1.drain_dirty(), w2.drain_dirty());
        w1.compact();
        w2.compact();
        assert_eq!(w1.chunk_count(), w2.chunk_count());
        assert_eq!(w1.uniform_chunk_count(), w2.uniform_chunk_count());
        // Reads through the octree fallback agree.
        let probe = WorldCoord { x: 0, y: 0, z: 0 };
        assert_eq!(w1.read(probe), w2.read(probe));
    }

    /// FR-PHENO-VOXEL-WORLD-005 — replaying the same write sequence on a fresh
    /// world produces bit-identical dirty events.
    #[test]
    fn replay_is_bit_identical() {
        let writes = [
            (
                WorldCoord {
                    x: 5_000_000,
                    y: 0,
                    z: 0,
                },
                1u8,
            ),
            (
                WorldCoord {
                    x: 0,
                    y: 5_000_000,
                    z: 0,
                },
                2u8,
            ),
            (
                WorldCoord {
                    x: 0,
                    y: 0,
                    z: 5_000_000,
                },
                3u8,
            ),
            (
                WorldCoord {
                    x: 5_000_000,
                    y: 5_000_000,
                    z: 5_000_000,
                },
                4u8,
            ),
        ];
        let mut w1: VoxelWorld<u8> = VoxelWorld::new(1_000_000);
        let mut w2: VoxelWorld<u8> = VoxelWorld::new(1_000_000);
        for (pos, v) in writes {
            w1.write(pos, v);
            w2.write(pos, v);
        }
        assert_eq!(w1.drain_dirty(), w2.drain_dirty());
    }

    /// FR-PHENO-VOXEL-WORLD-010 — `chunk()` returns `None` for an empty world.
    #[test]
    fn chunk_returns_none_for_empty_world() {
        let w: VoxelWorld<u8> = VoxelWorld::new(1_000_000);
        assert!(w
            .chunk(ChunkCoord {
                cx: 0,
                cy: 0,
                cz: 0
            })
            .is_none());
    }

    /// FR-PHENO-VOXEL-WORLD-011 — `chunk()` returns `Some` after a write.
    #[test]
    fn chunk_returns_some_after_write() {
        let mut w: VoxelWorld<u8> = VoxelWorld::new(1_000_000);
        let coord = w.write(WorldCoord { x: 0, y: 0, z: 0 }, 11);
        assert!(w.chunk(coord).is_some());
    }

    /// FR-PHENO-VOXEL-WORLD-012 — `chunk()` returns `None` after compaction
    /// promotes the dense chunk into the octree.
    #[test]
    fn chunk_returns_none_after_compaction() {
        let mut w: VoxelWorld<u8> = VoxelWorld::new(1_000_000);
        for z in 0..16 {
            for y in 0..16 {
                for x in 0..16 {
                    w.write(
                        WorldCoord {
                            x: x * 1_000_000,
                            y: y * 1_000_000,
                            z: z * 1_000_000,
                        },
                        3,
                    );
                }
            }
        }
        let coord = ChunkCoord {
            cx: 0,
            cy: 0,
            cz: 0,
        };
        assert!(w.chunk(coord).is_some());
        assert_eq!(w.compact(), 1);
        assert!(w.chunk(coord).is_none());
        assert_eq!(w.octree().uniform_value(coord), Some(3));
    }

    /// FR-PHENO-VOXEL-WORLD-013 — `chunks_dense()` iterates in `BTreeMap`
    /// coordinate order.
    #[test]
    fn chunks_dense_iterates_in_order() {
        let mut w: VoxelWorld<u8> = VoxelWorld::new(1_000_000);
        w.write(
            WorldCoord {
                x: 32_000_000,
                y: 0,
                z: 0,
            },
            1,
        );
        w.write(
            WorldCoord {
                x: 0,
                y: 32_000_000,
                z: 0,
            },
            2,
        );
        w.write(
            WorldCoord {
                x: 16_000_000,
                y: 0,
                z: 0,
            },
            3,
        );

        let coords: Vec<ChunkCoord> = w.chunks_dense().map(|(coord, _)| coord).collect();
        assert_eq!(
            coords,
            vec![
                ChunkCoord {
                    cx: 0,
                    cy: 2,
                    cz: 0
                },
                ChunkCoord {
                    cx: 1,
                    cy: 0,
                    cz: 0
                },
                ChunkCoord {
                    cx: 2,
                    cy: 0,
                    cz: 0
                },
            ]
        );
    }

    /// FR-PHENO-VOXEL-WORLD-015 — out-of-bounds / extreme coordinates never panic;
    /// they map to a valid chunk via euclidean-division and round-trip correctly.
    #[test]
    fn out_of_bounds_coords_do_not_panic() {
        let mut w: VoxelWorld<u8> = VoxelWorld::new(1_000_000);
        let extreme_coords = [
            WorldCoord {
                x: i64::MAX / 2,
                y: 0,
                z: 0,
            },
            WorldCoord {
                x: i64::MIN / 2,
                y: 0,
                z: 0,
            },
            WorldCoord {
                x: -1_000_000_000,
                y: -1_000_000_000,
                z: -1_000_000_000,
            },
            WorldCoord {
                x: 1_000_000_000,
                y: 1_000_000_000,
                z: 1_000_000_000,
            },
        ];
        for pos in extreme_coords {
            // Must not panic; the return value is a valid ChunkCoord.
            let coord = w.write(pos, 7u8);
            // Read back must equal what was written.
            assert_eq!(w.read(pos), 7, "round-trip failed at {pos:?}");
            // The dense chunk must exist.
            assert!(w.chunk(coord).is_some(), "chunk not allocated for {pos:?}");
        }
        // All four extreme writes produced exactly four chunks (different chunk-grid
        // coordinates) or fewer if any of them happen to share a chunk.
        assert!(w.chunk_count() >= 1 && w.chunk_count() <= 4);
    }

    /// FR-PHENO-VOXEL-WORLD-016 — multiple writes to the same chunk coexist
    /// independently; distinct voxels within one chunk each return their own value.
    #[test]
    fn multiple_writes_to_same_chunk_coexist() {
        let mut w: VoxelWorld<u8> = VoxelWorld::new(1_000_000);
        // All three positions are inside the 16³ chunk at origin (voxel_span=1e6).
        let p0 = WorldCoord { x: 0, y: 0, z: 0 };
        let p1 = WorldCoord {
            x: 1_000_000,
            y: 0,
            z: 0,
        };
        let p2 = WorldCoord {
            x: 0,
            y: 1_000_000,
            z: 0,
        };
        let p3 = WorldCoord {
            x: 0,
            y: 0,
            z: 1_000_000,
        };
        w.write(p0, 10);
        w.write(p1, 20);
        w.write(p2, 30);
        w.write(p3, 40);
        // All in the same chunk.
        assert_eq!(w.chunk_count(), 1, "expected a single chunk for all writes");
        // Each voxel retains its own value.
        assert_eq!(w.read(p0), 10);
        assert_eq!(w.read(p1), 20);
        assert_eq!(w.read(p2), 30);
        assert_eq!(w.read(p3), 40);
        // Overwriting one voxel must not disturb its neighbours.
        w.write(p0, 99);
        assert_eq!(w.read(p0), 99);
        assert_eq!(w.read(p1), 20);
        assert_eq!(w.read(p2), 30);
        assert_eq!(w.read(p3), 40);
    }

    /// FR-PHENO-VOXEL-WORLD-014 — `ChunkCoord::chunk_id()` matches the kernel's
    /// canonical packed chunk-ID encoding.
    #[test]
    fn chunk_coord_chunk_id_matches_kernel_packing() {
        let coord = ChunkCoord {
            cx: -1,
            cy: 0x1234_5678,
            cz: -2,
        };
        let expected = crate::chunk::ChunkId(
            (((coord.cx as u32) as u64) << 40)
                | (((coord.cy as u32) as u64) << 16)
                | (((coord.cz as u32) as u64) & 0xFFFF),
        );
        assert_eq!(coord.chunk_id(), expected);

        let mut w: VoxelWorld<u8> = VoxelWorld::new(1_000_000);
        w.write(
            WorldCoord {
                x: -1_000_000,
                y: 0,
                z: 0,
            },
            9,
        );
        assert_eq!(
            w.drain_dirty()[0].chunk_id,
            ChunkCoord {
                cx: -1,
                cy: 0,
                cz: 0
            }
            .chunk_id()
        );
    }
}
