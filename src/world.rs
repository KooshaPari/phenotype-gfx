//! Top-level `VoxelWorld`: deterministic write + drain over the hybrid SVO + dense
//! leaf-chunk storage.
//!
//! P-V1 milestone: only dense-leaf writes are implemented end-to-end; SVO-level
//! sparse upgrades land in a follow-up PR. The public API is shaped so that future
//! sparse upgrades are additive (callers always go through [`VoxelWorld::write`]
//! and [`VoxelWorld::drain_dirty`]).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::chunk::{Chunk, ChunkId, CHUNK_EDGE, CHUNK_VOXELS};
use crate::coord::{to_chunk_coord, ChunkCoord, WorldCoord};
use crate::delta::{DirtyChunkEvent, WriteSeq};

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
                chunk_id: chunk_id_for(coord),
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

    /// Read a voxel at the given world position. Returns the default value when
    /// the containing chunk has not been allocated yet.
    #[must_use]
    pub fn read(&self, pos: WorldCoord) -> T {
        let coord = to_chunk_coord(pos, self.voxel_span, CHUNK_EDGE as i32);
        let Some(chunk) = self.chunks.get(&coord) else {
            return T::default();
        };
        let edge = CHUNK_EDGE as i64;
        let world_edge = self.voxel_span * edge;
        let local_x = pos.x.rem_euclid(world_edge).div_euclid(self.voxel_span) as usize;
        let local_y = pos.y.rem_euclid(world_edge).div_euclid(self.voxel_span) as usize;
        let local_z = pos.z.rem_euclid(world_edge).div_euclid(self.voxel_span) as usize;
        let idx = local_x + local_y * CHUNK_EDGE + local_z * CHUNK_EDGE * CHUNK_EDGE;
        chunk.voxels[idx].clone()
    }

    /// Number of allocated chunks. Test-friendly observability.
    #[must_use]
    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }
}

/// Stable per-`ChunkCoord` [`ChunkId`]. Packs the signed grid coordinates into a
/// single `u64` so dirty events can be ordered without leaking BTreeMap ordering.
fn chunk_id_for(c: ChunkCoord) -> ChunkId {
    // Treat the i32 components as u32 bit-patterns so the resulting ID is unique
    // for every distinct (cx, cy, cz) triple. Truncate Z to the bottom byte because
    // 24+24+16=64 fits in a u64 but the planet-scale world is heavily wider on the
    // XY plane than Z. This is the same encoding choice as Minecraft's chunk hash.
    let cx = (c.cx as u32) as u64;
    let cy = (c.cy as u32) as u64;
    let cz = (c.cz as u32) as u64;
    ChunkId((cx << 40) | (cy << 16) | (cz & 0xFFFF))
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
}
