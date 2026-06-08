//! Chunk adapter: [`DenseChunkStore`] implements the [`Chunkable`] port.

use std::collections::HashMap;

use crate::ports::chunk::{ChunkId, ChunkView, Chunkable};
use crate::chunk::Chunk;

/// Dense chunk storage keyed by [`ChunkId`].
#[derive(Debug, Clone)]
pub struct DenseChunkStore<T: Default + Clone> {
    chunks: HashMap<ChunkId, Chunk<T>>,
}

impl<T: Default + Clone> Default for DenseChunkStore<T> {
    fn default() -> Self {
        Self { chunks: HashMap::new() }
    }
}

impl<T: Default + Clone> DenseChunkStore<T> {
    /// Insert a chunk into the store.
    pub fn insert(&mut self, id: ChunkId, chunk: Chunk<T>) {
        self.chunks.insert(id, chunk);
    }

    /// Remove a chunk from the store.
    pub fn remove(&mut self, id: ChunkId) -> Option<Chunk<T>> {
        self.chunks.remove(&id)
    }

    /// Get a reference to a chunk.
    pub fn get(&self, id: ChunkId) -> Option<&Chunk<T>> {
        self.chunks.get(&id)
    }

    /// Get a mutable reference to a chunk.
    pub fn get_mut(&mut self, id: ChunkId) -> Option<&mut Chunk<T>> {
        self.chunks.get_mut(&id)
    }

    /// Number of chunks currently stored.
    pub fn len(&self) -> usize {
        self.chunks.len()
    }

    /// True when no chunks are stored.
    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    /// Get or insert a default chunk.
    pub fn get_or_insert(&mut self, id: ChunkId) -> &mut Chunk<T> {
        self.chunks.entry(id).or_default()
    }
}

impl<T: Default + Clone> Chunkable for DenseChunkStore<T> {
    type Voxel = T;

    fn view(&self, id: ChunkId) -> Option<ChunkView<'_, T>> {
        self.chunks.get(&id).map(|c| ChunkView {
            id,
            voxels: &c.voxels,
        })
    }
}
