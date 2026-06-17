//! Reusable test fixtures for [`DirtyChunkEvent`] ordering and replay.
//!
//! These fixtures codify the event-order matrix described in
//! `specs/dirty-chunk-ordering-fixure.md`.  They are exported (under
//! `pub mod fixtures`) so downstream consumers (Civis, WorldSphereMod3D, and
//! future game integrations) can pin the same input vectors in their own
//! integration tests instead of hand-rolling matrices that drift over time.
//!
//! Three families are exposed:
//!
//! * [`sample_sequence`] — a single, well-known write sequence that mixes
//!   chunks and sequence numbers; used to assert `(chunk_id, write_seq)`
//!   sort order is stable.
//! * [`sample_with_ties`] — a sequence containing two events with identical
//!   `(chunk_id, write_seq)` values; consumers must treat these as a no-op
//!   and not double-rebuild the chunk.
//! * [`sample_lod_transition`] — a sequence that interleaves writes from
//!   two adjacent LOD levels on the same chunk; ordering must be preserved
//!   across the transition so LOD demotion/promotion does not desync
//!   rebuild order.

use crate::chunk::ChunkId;
use crate::delta::{DirtyChunkEvent, WriteSeq};

/// A canonical write sequence used by the sort-by-(chunk_id, write_seq) test.
///
/// Ordering after `sort()`:
/// `(ChunkId(1), WriteSeq(3))`, `(ChunkId(1), WriteSeq(10))`,
/// `(ChunkId(2), WriteSeq(1))`, `(ChunkId(2), WriteSeq(5))`.
pub fn sample_sequence() -> Vec<DirtyChunkEvent> {
    vec![
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
    ]
}

/// A sequence with two events sharing the exact same `(chunk_id, write_seq)`.
///
/// Consumers must treat the duplicate as a no-op (the second event carries
/// no new information).  The fixture intentionally leaves the duplicate
/// inline so the test can prove the no-op policy without sneaking it in
/// after the fact.
pub fn sample_with_ties() -> Vec<DirtyChunkEvent> {
    vec![
        DirtyChunkEvent {
            chunk_id: ChunkId(7),
            write_seq: WriteSeq(2),
        },
        DirtyChunkEvent {
            chunk_id: ChunkId(3),
            write_seq: WriteSeq(1),
        },
        // Duplicate of the first event below — must be deduped by consumers.
        DirtyChunkEvent {
            chunk_id: ChunkId(7),
            write_seq: WriteSeq(2),
        },
        DirtyChunkEvent {
            chunk_id: ChunkId(3),
            write_seq: WriteSeq(1),
        },
    ]
}

/// A sequence that interleaves writes from two adjacent LOD levels on
/// the same chunk id.  The fixture uses *different* chunk ids (101 / 102)
/// for the two LOD bands because the substrate's chunk id is currently
/// LOD-agnostic; the spec calls out that the order between events must
/// remain stable across the LOD transition.
///
/// The named constant [`LOD_LOWER_CHUNK`] and [`LOD_UPPER_CHUNK`] keep
/// the two ids discoverable from downstream code.
pub const LOD_LOWER_CHUNK: ChunkId = ChunkId(101);
/// Upper neighbour id used by [`sample_lod_transition`] to exercise the
/// cross-LOD dirty ordering path on the consumer side.
pub const LOD_UPPER_CHUNK: ChunkId = ChunkId(102);

/// Interleaved write sequence across [`LOD_LOWER_CHUNK`] and
/// [`LOD_UPPER_CHUNK`] used to verify the consumer's sort+dedup policy
/// preserves order across a LOD transition.
pub fn sample_lod_transition() -> Vec<DirtyChunkEvent> {
    vec![
        DirtyChunkEvent {
            chunk_id: LOD_LOWER_CHUNK,
            write_seq: WriteSeq(1),
        },
        DirtyChunkEvent {
            chunk_id: LOD_UPPER_CHUNK,
            write_seq: WriteSeq(2),
        },
        DirtyChunkEvent {
            chunk_id: LOD_LOWER_CHUNK,
            write_seq: WriteSeq(3),
        },
        DirtyChunkEvent {
            chunk_id: LOD_UPPER_CHUNK,
            write_seq: WriteSeq(4),
        },
    ]
}

/// Stable sort + dedup-by-key convenience used by consumer test code.
///
/// Returns the input sorted by `(chunk_id, write_seq)` with consecutive
/// duplicates removed.  This is *not* a general-purpose dedup; it is the
/// exact policy the substrate recommends consumers adopt.
pub fn sort_and_dedup(events: Vec<DirtyChunkEvent>) -> Vec<DirtyChunkEvent> {
    let mut sorted = events;
    sorted.sort();
    sorted.dedup();
    sorted
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fixture re-export smoke: the sample sequence must be non-empty and
    /// must contain at least two distinct chunk ids so sort-order matters.
    #[test]
    fn sample_sequence_is_meaningful() {
        let s = sample_sequence();
        assert!(s.len() >= 2, "sample must have >= 2 events");
        let mut chunks: Vec<_> = s.iter().map(|e| e.chunk_id).collect();
        chunks.sort();
        chunks.dedup();
        assert!(chunks.len() >= 2, "sample must span >= 2 chunks");
    }

    /// Tie policy: dedup must collapse the duplicate (ChunkId(7), WriteSeq(2))
    /// to a single event.
    #[test]
    fn dedup_collapses_ties() {
        let deduped = sort_and_dedup(sample_with_ties());
        let count_7_2 = deduped
            .iter()
            .filter(|e| e.chunk_id == ChunkId(7) && e.write_seq == WriteSeq(2))
            .count();
        assert_eq!(count_7_2, 1, "duplicate (7, 2) should collapse to 1");
        let count_3_1 = deduped
            .iter()
            .filter(|e| e.chunk_id == ChunkId(3) && e.write_seq == WriteSeq(1))
            .count();
        assert_eq!(count_3_1, 1, "duplicate (3, 1) should collapse to 1");
    }

    /// LOD transition: the relative order of the two chunk ids must be
    /// preserved after sort, regardless of which LOD was written first.
    #[test]
    fn lod_transition_preserves_order() {
        let sorted = sort_and_dedup(sample_lod_transition());
        let lower_idx = sorted
            .iter()
            .position(|e| e.chunk_id == LOD_LOWER_CHUNK)
            .expect("lower chunk present");
        let upper_idx = sorted
            .iter()
            .position(|e| e.chunk_id == LOD_UPPER_CHUNK)
            .expect("upper chunk present");
        // The lower-LOD chunk appears first in the input; after sort
        // (lower id first) it must still appear before the upper-LOD
        // chunk in the output.
        assert!(
            lower_idx < upper_idx,
            "LOD_LOWER_CHUNK (id=101) must sort before LOD_UPPER_CHUNK (id=102)"
        );
    }
}
