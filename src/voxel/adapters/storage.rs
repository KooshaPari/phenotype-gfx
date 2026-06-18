//! Storage adapter: [`VoxelWorldAdapter`] implements the [`WorldStore`] port on
//! top of the canonical domain [`VoxelWorld`].
//!
//! The adapter is a thin zero-cost wrapper: every method forwards to the
//! underlying [`VoxelWorld`]. The point of the adapter is to provide a name
//! that the rest of the codebase (and downstream crates) can import as
//! "the canonical chunked + sparse-octree storage backend" without having to
//! depend on the `VoxelWorld` concrete type — preserving the hexagon
//! boundary defined in [`crate::voxel::ports::storage`].

use crate::voxel::chunk::Chunk;
use crate::voxel::coord::{ChunkCoord, WorldCoord};
use crate::voxel::delta::DirtyChunkEvent;
use crate::voxel::ports::storage::WorldStore;
use crate::voxel::world::VoxelWorld;

/// Adapter that exposes a [`VoxelWorld`] through the [`WorldStore`] port.
///
/// This is the production-grade in-memory backend: dense leaf chunks + a
/// sparse octree, with the same write-then-compact flow documented on
/// [`VoxelWorld`]. Use this when you need chunked storage, octree promotion,
/// and replay-safe dirty-event draining.
#[derive(Debug, Clone)]
pub struct VoxelWorldAdapter<T: Default + Clone + PartialEq> {
    inner: VoxelWorld<T>,
    /// Cached copy of the voxel's fixed-point world-unit span. [`VoxelWorld`]
    /// takes this at construction but does not expose a getter, so the
    /// adapter shadows it locally to satisfy [`WorldStore::voxel_span`].
    voxel_span: i64,
}

impl<T: Default + Clone + PartialEq> VoxelWorldAdapter<T> {
    /// Construct an adapter around a fresh [`VoxelWorld`] with the given
    /// `voxel_span` (fixed-point world units per voxel edge).
    pub fn with_voxel_span(voxel_span: i64) -> Self {
        Self {
            inner: VoxelWorld::new(voxel_span),
            voxel_span,
        }
    }

    /// Wrap an existing [`VoxelWorld`]. The adapter's `voxel_span` mirrors
    /// whatever the wrapped world was constructed with; the caller is
    /// responsible for keeping them in sync. This constructor exists for
    /// completeness in case future refactors expose a getter on
    /// [`VoxelWorld`].
    pub fn new(world: VoxelWorld<T>) -> Self {
        // VoxelWorld has no public `voxel_span` getter, so we cannot recover
        // the span from `world`. Constructing via `with_voxel_span` is the
        // recommended entry point.
        Self {
            inner: world,
            voxel_span: 0,
        }
    }

    /// Consume the adapter and return the underlying [`VoxelWorld`].
    pub fn into_inner(self) -> VoxelWorld<T> {
        self.inner
    }

    /// Borrow the underlying [`VoxelWorld`].
    pub fn inner(&self) -> &VoxelWorld<T> {
        &self.inner
    }

    /// Mutably borrow the underlying [`VoxelWorld`].
    pub fn inner_mut(&mut self) -> &mut VoxelWorld<T> {
        &mut self.inner
    }
}

impl<T: Default + Clone + PartialEq> WorldStore<T> for VoxelWorldAdapter<T> {
    fn read(&self, pos: WorldCoord) -> T {
        self.inner.read(pos)
    }

    fn write(&mut self, pos: WorldCoord, value: T) -> ChunkCoord {
        self.inner.write(pos, value)
    }

    fn drain_dirty(&mut self) -> Vec<DirtyChunkEvent> {
        self.inner.drain_dirty()
    }

    fn compact(&mut self) -> usize {
        self.inner.compact()
    }

    fn chunk_count(&self) -> usize {
        self.inner.chunk_count()
    }

    fn uniform_chunk_count(&self) -> usize {
        self.inner.uniform_chunk_count()
    }

    fn voxel_span(&self) -> i64 {
        self.voxel_span
    }

    fn chunk(&self, coord: ChunkCoord) -> Option<&Chunk<T>> {
        self.inner.chunk(coord)
    }

    fn chunks_dense(&self) -> Box<dyn Iterator<Item = (ChunkCoord, &Chunk<T>)> + '_> {
        // `VoxelWorld::chunks_dense` returns an opaque `impl Iterator`; we
        // adapt it to a `Box<dyn Iterator>` for object-safety on the port.
        Box::new(self.inner.chunks_dense())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voxel::coord::FIXED_SCALE;

    /// FR-PHENO-VOXEL-PORT-STORAGE-ADAPTER-000 — write/read round-trip through
    /// the adapter preserves voxel values.
    #[test]
    fn write_read_roundtrip() {
        let mut store = VoxelWorldAdapter::<u8>::with_voxel_span(FIXED_SCALE);
        let pos = WorldCoord {
            x: 5 * FIXED_SCALE,
            y: 0,
            z: 0,
        };
        let coord = store.write(pos, 9);
        assert_eq!(store.read(pos), 9);
        assert_eq!(store.chunk_count(), 1);
        assert_eq!(store.chunk(coord).map(|c| c.voxels.len()), Some(4096));
    }

    /// FR-PHENO-VOXEL-PORT-STORAGE-ADAPTER-001 — drain_dirty returns events in
    /// `(chunk_id, write_seq)` order regardless of write interleaving.
    #[test]
    fn drain_dirty_is_sorted() {
        let mut store = VoxelWorldAdapter::<u8>::with_voxel_span(FIXED_SCALE);
        let a0 = WorldCoord { x: 0, y: 0, z: 0 };
        let b0 = WorldCoord {
            x: 100 * FIXED_SCALE,
            y: 0,
            z: 0,
        };
        store.write(a0, 1);
        store.write(b0, 1);
        store.write(a0, 2);
        let dirty = store.drain_dirty();
        assert_eq!(dirty.len(), 3);
        for w in dirty.windows(2) {
            if w[0].chunk_id == w[1].chunk_id {
                assert!(w[0].write_seq < w[1].write_seq);
            } else {
                assert!(w[0].chunk_id < w[1].chunk_id);
            }
        }
    }

    /// FR-PHENO-VOXEL-PORT-STORAGE-ADAPTER-002 — `compact` promotes a
    /// uniform chunk into the sparse tier; `uniform_chunk_count` reflects
    /// the promotion; `chunks_dense` no longer yields it.
    #[test]
    fn compact_promotes_uniform_chunk() {
        let mut store = VoxelWorldAdapter::<u8>::with_voxel_span(FIXED_SCALE);
        // Fill the origin 16³ chunk uniformly with 7.
        for z in 0..16 {
            for y in 0..16 {
                for x in 0..16 {
                    store.write(
                        WorldCoord {
                            x: x * FIXED_SCALE,
                            y: y * FIXED_SCALE,
                            z: z * FIXED_SCALE,
                        },
                        7,
                    );
                }
            }
        }
        assert_eq!(store.chunk_count(), 1);
        assert_eq!(store.compact(), 1);
        assert_eq!(store.chunk_count(), 0);
        assert_eq!(store.uniform_chunk_count(), 1);
        // Read still works via the octree fallback.
        assert_eq!(store.read(WorldCoord { x: 0, y: 0, z: 0 }), 7);
        // chunks_dense is now empty.
        assert_eq!(store.chunks_dense().count(), 0);
    }

    /// FR-PHENO-VOXEL-PORT-STORAGE-ADAPTER-003 — `chunks_dense` iterates the
    /// dense chunk store in `BTreeMap` order (deterministic).
    #[test]
    fn chunks_dense_iterates_in_order() {
        let mut store = VoxelWorldAdapter::<u8>::with_voxel_span(FIXED_SCALE);
        store.write(
            WorldCoord {
                x: 32 * FIXED_SCALE,
                y: 0,
                z: 0,
            },
            1,
        );
        store.write(
            WorldCoord {
                x: 0,
                y: 32 * FIXED_SCALE,
                z: 0,
            },
            2,
        );
        store.write(
            WorldCoord {
                x: 16 * FIXED_SCALE,
                y: 0,
                z: 0,
            },
            3,
        );
        let coords: Vec<ChunkCoord> = store.chunks_dense().map(|(c, _)| c).collect();
        // (cy=2) < (cx=1) < (cx=2) under (cx, cy, cz) lex order.
        assert_eq!(coords[0].cy, 2);
        assert_eq!(coords[1].cx, 1);
        assert_eq!(coords[2].cx, 2);
    }
}
