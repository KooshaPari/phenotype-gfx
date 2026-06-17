//! Storage port: engine-neutral voxel-world storage contracts.
//!
//! The domain depends **only** on the [`WorldStore`] trait. Concrete storage
//! backends (in-memory chunked world, flat `HashMap`, file-backed world,
//! networked world, …) implement the same trait. The hexagon boundary is:
//!
//! ```text
//!   ┌─────────────────────────────┐
//!   │  domain (uses trait)        │ ── no concrete adapter import ──▶
//!   └─────────────┬───────────────┘
//!                 ▼
//!            WorldStore        ◀── port (this file)
//!                 ▲
//!   ┌─────────────┴───────────────┐
//!   │  adapters: VoxelWorld / IO  │
//!   └─────────────────────────────┘
//! ```

use thiserror::Error;

use crate::chunk::Chunk;
use crate::coord::{ChunkCoord, WorldCoord};
use crate::delta::{DirtyChunkEvent, WriteSeq};

// ────────────────────────────────────────────────────────────────────────────
// Errors
// ────────────────────────────────────────────────────────────────────────────

/// Errors that can be raised by a [`WorldStore`] adapter.
///
/// The current in-memory adapters cannot fail in normal use, but the trait
/// reserves this error so future backends (file, network, …) can return
/// domain-meaningful failures without forcing the trait to change shape.
#[derive(Debug, Error)]
pub enum StorageError {
    /// Catch-all for backend-specific failures that don't fit a richer
    /// variant.
    #[error("storage backend error: {0}")]
    Backend(String),
}

/// Result alias for storage port operations.
pub type StorageResult<T> = Result<T, StorageError>;

// ────────────────────────────────────────────────────────────────────────────
// Port trait
// ────────────────────────────────────────────────────────────────────────────

/// Hexagonal port: engine-neutral voxel-world storage.
///
/// Every concrete storage backend — in-memory chunked
/// [`VoxelWorld`](crate::world::VoxelWorld), flat `HashMap` map, file-backed
/// world, networked world — implements this trait. The domain code imports
/// **only** this trait; never a concrete adapter.
///
/// Type parameter `T` is the voxel value type. `T: PartialEq` is required
/// for the idempotent-write contract: a re-write with the same value must
/// not produce a dirty event, which is detected via `==` comparison.
pub trait WorldStore<T: Default + Clone + PartialEq> {
    /// Read a voxel at the given world position. Returns the default value of
    /// `T` for any coord the backend has no record of.
    fn read(&self, pos: WorldCoord) -> T;

    /// Write a voxel at the given world position. Returns the chunk-grid
    /// coordinate of the chunk that owns `pos` so the caller can emit a
    /// [`DirtyChunkEvent`].
    ///
    /// Implementations are encouraged to be idempotent — re-writing the same
    /// value must not produce a dirty event.
    fn write(&mut self, pos: WorldCoord, value: T) -> ChunkCoord;

    /// Drain all pending dirty events in deterministic `(chunk_id, write_seq)`
    /// order. Returns an empty vector when no writes are pending.
    fn drain_dirty(&mut self) -> Vec<DirtyChunkEvent>;

    /// Compact the world by promoting uniform dense chunks into a sparse
    /// representation. Returns the number of chunks promoted in this pass.
    /// The operation must be idempotent — re-running on a fully-compacted
    /// world must return `0`.
    fn compact(&mut self) -> usize;

    /// Number of *dense* chunks currently held by the backend.
    fn chunk_count(&self) -> usize;

    /// Number of chunks promoted to the sparse tier (uniform regions).
    fn uniform_chunk_count(&self) -> usize;

    /// Number of fixed-point world units that one voxel edge spans.
    fn voxel_span(&self) -> i64;

    /// Borrow a dense chunk by chunk-grid coordinate, if present.
    fn chunk(&self, coord: ChunkCoord) -> Option<&Chunk<T>>;

    /// Iterate all dense chunks in deterministic order (replay-safe by
    /// construction — backends must use a `BTreeMap`-like order source).
    fn chunks_dense(&self) -> Box<dyn Iterator<Item = (ChunkCoord, &Chunk<T>)> + '_>;
}

// ────────────────────────────────────────────────────────────────────────────
// Test mock
// ────────────────────────────────────────────────────────────────────────────

/// One call recorded by [`MockWorldStore`].
#[derive(Debug, Clone, PartialEq)]
pub enum MockStoreCall {
    /// `read(pos)` was invoked.
    Read(WorldCoord),
    /// `write(pos, value)` was invoked. The value itself is **not** recorded
    /// (the trait bound is `T: Default + Clone`, which does not require
    /// `Debug`); tests that need to assert on the written value should call
    /// `record_read(pos)` from a `&mut self` context after the write.
    Write(WorldCoord),
    /// `drain_dirty()` was invoked.
    DrainDirty,
    /// `compact()` was invoked.
    Compact,
}

/// Recording mock used by domain tests to assert storage interaction order.
///
/// Stores every `read` and `write` call into a `Vec` so a test can replay the
/// sequence and verify the domain did not skip steps. The underlying voxel
/// state is a `Vec<T>` shadowing a flat `HashMap<WorldCoord, T>` for
/// read/write/iteration. The mock never produces a `StorageError`.
#[derive(Debug, Clone)]
pub struct MockWorldStore<T: Default + Clone + PartialEq> {
    voxels: std::collections::HashMap<WorldCoord, T>,
    voxel_span: i64,
    dirty: Vec<DirtyChunkEvent>,
    write_seq: WriteSeq,
    calls: Vec<MockStoreCall>,
}

impl<T: Default + Clone + PartialEq> MockWorldStore<T> {
    /// Construct a new empty mock store with the given `voxel_span`.
    pub fn new(voxel_span: i64) -> Self {
        Self {
            voxels: std::collections::HashMap::new(),
            voxel_span,
            dirty: Vec::new(),
            write_seq: WriteSeq::default(),
            calls: Vec::new(),
        }
    }

    /// Returns the recorded call sequence.
    pub fn calls(&self) -> &[MockStoreCall] {
        &self.calls
    }

    /// Reset the recorded calls (keeps voxel state intact).
    pub fn reset_calls(&mut self) {
        self.calls.clear();
    }

    /// Number of distinct voxel positions currently held.
    pub fn voxel_count(&self) -> usize {
        self.voxels.len()
    }
}

impl<T: Default + Clone + PartialEq> Default for MockWorldStore<T> {
    fn default() -> Self {
        Self::new(crate::coord::FIXED_SCALE)
    }
}

impl<T: Default + Clone + PartialEq> WorldStore<T> for MockWorldStore<T> {
    fn read(&self, pos: WorldCoord) -> T {
        // Recording in `&self` would require interior mutability; tests that
        // need a read-recording assertion should call `record_read` from a
        // `&mut self` context (mirrors the `MockMaterialRegistry` pattern).
        self.voxels.get(&pos).cloned().unwrap_or_default()
    }

    fn write(&mut self, pos: WorldCoord, value: T) -> ChunkCoord {
        self.calls.push(MockStoreCall::Write(pos));
        let coord =
            crate::coord::to_chunk_coord(pos, self.voxel_span, crate::chunk::CHUNK_EDGE as i32);
        let existing = self.voxels.get(&pos).cloned();
        if existing.as_ref() != Some(&value) {
            self.voxels.insert(pos, value);
            self.dirty.push(DirtyChunkEvent {
                chunk_id: coord.chunk_id(),
                write_seq: self.write_seq.advance(),
            });
        }
        coord
    }

    fn drain_dirty(&mut self) -> Vec<DirtyChunkEvent> {
        self.calls.push(MockStoreCall::DrainDirty);
        let mut out = std::mem::take(&mut self.dirty);
        out.sort();
        out
    }

    fn compact(&mut self) -> usize {
        self.calls.push(MockStoreCall::Compact);
        // Mock has no sparse tier — always a no-op.
        0
    }

    fn chunk_count(&self) -> usize {
        // Count distinct chunk-grid coordinates the mock has touched.
        let mut seen = std::collections::HashSet::new();
        for pos in self.voxels.keys() {
            seen.insert(crate::coord::to_chunk_coord(
                *pos,
                self.voxel_span,
                crate::chunk::CHUNK_EDGE as i32,
            ));
        }
        seen.len()
    }

    fn uniform_chunk_count(&self) -> usize {
        0
    }

    fn voxel_span(&self) -> i64 {
        self.voxel_span
    }

    fn chunk(&self, coord: ChunkCoord) -> Option<&Chunk<T>> {
        // The flat mock has no first-class chunks to return. Callers that
        // need chunk iteration should use `chunks_dense` (empty here) and
        // fall back to position-level reads via `read`.
        let _ = coord;
        None
    }

    fn chunks_dense(&self) -> Box<dyn Iterator<Item = (ChunkCoord, &Chunk<T>)> + '_> {
        Box::new(std::iter::empty())
    }
}

impl<T: Default + Clone + PartialEq> MockWorldStore<T> {
    /// Record a `read` call from a `&mut self` context (used by tests).
    pub fn record_read(&mut self, pos: WorldCoord) -> T {
        self.calls.push(MockStoreCall::Read(pos));
        self.voxels.get(&pos).cloned().unwrap_or_default()
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Unit tests
// ────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// FR-PHENO-VOXEL-PORT-STORAGE-000 — write+read round-trip returns the
    /// same value and produces a single dirty event.
    #[test]
    fn mock_write_read_roundtrip() {
        let mut store = MockWorldStore::<u8>::new(crate::coord::FIXED_SCALE);
        let pos = WorldCoord { x: 0, y: 0, z: 0 };
        let coord = store.write(pos, 7);
        assert_eq!(store.read(pos), 7);
        assert_eq!(store.voxel_count(), 1);
        let dirty = store.drain_dirty();
        assert_eq!(dirty.len(), 1);
        assert_eq!(dirty[0].chunk_id, coord.chunk_id());
    }

    /// FR-PHENO-VOXEL-PORT-STORAGE-001 — idempotent writes (same value) do
    /// not produce a second dirty event.
    #[test]
    fn mock_idempotent_write_emits_no_event() {
        let mut store = MockWorldStore::<u8>::new(crate::coord::FIXED_SCALE);
        let pos = WorldCoord { x: 0, y: 0, z: 0 };
        store.write(pos, 7);
        let _ = store.drain_dirty();
        store.write(pos, 7);
        assert!(store.drain_dirty().is_empty());
    }

    /// FR-PHENO-VOXEL-PORT-STORAGE-002 — `compact` is a no-op for the flat
    /// mock and records the call.
    #[test]
    fn mock_compact_is_noop() {
        let mut store = MockWorldStore::<u8>::new(crate::coord::FIXED_SCALE);
        assert_eq!(store.compact(), 0);
        assert!(store
            .calls()
            .iter()
            .any(|c| matches!(c, MockStoreCall::Compact)));
    }

    /// FR-PHENO-VOXEL-PORT-STORAGE-003 — `record_read` populates the call log
    /// and returns the stored value (or default when absent).
    #[test]
    fn mock_record_read_returns_value_or_default() {
        let mut store = MockWorldStore::<u8>::new(crate::coord::FIXED_SCALE);
        let pos = WorldCoord { x: 0, y: 0, z: 0 };
        store.write(pos, 11);
        store.reset_calls();
        let got = store.record_read(pos);
        assert_eq!(got, 11);
        assert_eq!(store.calls(), &[MockStoreCall::Read(pos)]);

        // An absent coord returns the default.
        let missing = WorldCoord {
            x: 100_000_000,
            y: 0,
            z: 0,
        };
        let got_missing = store.record_read(missing);
        assert_eq!(got_missing, 0);
        assert_eq!(
            store.calls(),
            &[MockStoreCall::Read(pos), MockStoreCall::Read(missing)]
        );
    }
}
