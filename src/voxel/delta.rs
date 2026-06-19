//! Deterministic dirty-event queue.
//!
//! Every voxel write must produce a [`DirtyChunkEvent`] tagged with a monotonically
//! increasing [`WriteSeq`]. Consumers drain events in `(chunk_id, write_seq)` order
//! so chunk-mesh rebuild order is bit-identical across machines and replays.

use serde::{Deserialize, Serialize};

use crate::voxel::chunk::ChunkId;

/// Monotonic write sequence number. Wraps `u64`; one per voxel-world write op.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
pub struct WriteSeq(pub u64);

impl WriteSeq {
    /// Advance the sequence by one and return the previous value. Named
    /// `advance` rather than `next` so it does not collide with
    /// [`Iterator::next`].
    #[must_use = "the returned value is the seq to attach to the write that just happened"]
    pub fn advance(&mut self) -> Self {
        let v = *self;
        self.0 = self.0.wrapping_add(1);
        v
    }
}

/// Event emitted when a chunk has been modified and needs re-meshing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DirtyChunkEvent {
    /// Stable ID of the affected chunk.
    pub chunk_id: ChunkId,
    /// Write sequence at which this dirty event was emitted.
    pub write_seq: WriteSeq,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// FR-PHENO-VOXEL-DELTA-000 — `WriteSeq::next` is monotonic and one-shot.
    #[test]
    fn writeseq_next_is_monotonic() {
        let mut s = WriteSeq::default();
        let a = s.advance();
        let b = s.advance();
        let c = s.advance();
        assert_eq!(a.0, 0);
        assert_eq!(b.0, 1);
        assert_eq!(c.0, 2);
        assert!(a < b && b < c);
    }

    /// FR-PHENO-VOXEL-DELTA-001 — events sort by `(chunk_id, write_seq)` so consumer
    /// iteration is deterministic regardless of how the producer batched them.
    #[test]
    fn events_sort_deterministically() {
        let evts = vec![
            DirtyChunkEvent {
                chunk_id: ChunkId(2),
                write_seq: WriteSeq(5),
            },
            DirtyChunkEvent {
                chunk_id: ChunkId(1),
                write_seq: WriteSeq(10),
            },
            DirtyChunkEvent {
                chunk_id: ChunkId(1),
                write_seq: WriteSeq(3),
            },
            DirtyChunkEvent {
                chunk_id: ChunkId(2),
                write_seq: WriteSeq(1),
            },
        ];
        let mut sorted = evts.clone();
        sorted.sort();
        assert_eq!(sorted[0].chunk_id, ChunkId(1));
        assert_eq!(sorted[0].write_seq, WriteSeq(3));
        assert_eq!(sorted[1].chunk_id, ChunkId(1));
        assert_eq!(sorted[1].write_seq, WriteSeq(10));
        assert_eq!(sorted[2].chunk_id, ChunkId(2));
        assert_eq!(sorted[2].write_seq, WriteSeq(1));
        assert_eq!(sorted[3].chunk_id, ChunkId(2));
        assert_eq!(sorted[3].write_seq, WriteSeq(5));
    }
}
