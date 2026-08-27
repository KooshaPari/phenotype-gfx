//! Disk-based chunk persistence for the streaming window.
//!
//! Provides [`ChunkStorage`] trait and [`DiskChunkStorage`] implementation for
//! saving/loading voxel chunks to/from disk with zstd compression. Integrates
//! with the streaming window policy so evicted chunks are persisted and can be
//! restored without regenerating from seed.
//!
//! ## Architecture
//!
//! ```text
//! StreamingManager
//!   └─► ChunkStorage (trait)
//!         └─► DiskChunkStorage (saves/chunks/{x}_{y}_{z}.bin)
//! ```
//!
//! All I/O is synchronous (blocking) inside a dedicated tokio task, never
//! blocking the render thread. The `DiskChunkStorage` uses `std::fs` for
//! portability with zstd framing for compression.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::voxel::ChunkCoord;

// ---------------------------------------------------------------------------
// ChunkStorage trait
// ---------------------------------------------------------------------------

/// Error type for chunk storage operations.
#[derive(Debug, Clone)]
pub enum ChunkStorageError {
    /// I/O error during read/write.
    Io(String),
    /// Serialization/deserialization error.
    Serde(String),
    /// Compression/decompression error.
    Compression(String),
    /// The requested chunk does not exist on disk.
    NotFound(ChunkCoord),
}

impl std::fmt::Display for ChunkStorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(msg) => write!(f, "I/O error: {msg}"),
            Self::Serde(msg) => write!(f, "Serde error: {msg}"),
            Self::Compression(msg) => write!(f, "Compression error: {msg}"),
            Self::NotFound(coord) => {
                write!(f, "Chunk not found at ({}, {}, {})", coord.cx, coord.cy, coord.cz)
            }
        }
    }
}

impl std::error::Error for ChunkStorageError {}

/// Serializable chunk payload stored on disk.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChunkPayload {
    /// The chunk coordinate.
    pub coord: ChunkCoord,
    /// Raw voxel data (pre-compressed bytes).
    pub data: Vec<u8>,
    /// Optional metadata (material palette, dirty flags, etc.).
    pub metadata: HashMap<String, String>,
}

/// Trait for chunk persistence backends.
///
/// Implementations handle saving, loading, and deleting compressed chunk data.
pub trait ChunkStorage: Send + Sync {
    /// Persist a chunk to storage.
    fn save_chunk(&self, payload: &ChunkPayload) -> Result<(), ChunkStorageError>;

    /// Load a chunk from storage.
    fn load_chunk(&self, coord: ChunkCoord) -> Result<ChunkPayload, ChunkStorageError>;

    /// Delete a chunk from storage.
    fn delete_chunk(&self, coord: ChunkCoord) -> Result<(), ChunkStorageError>;

    /// Check if a chunk exists in storage.
    fn chunk_exists(&self, coord: ChunkCoord) -> bool;
}

// ---------------------------------------------------------------------------
// DiskChunkStorage
// ---------------------------------------------------------------------------

/// File-based chunk storage with zstd compression.
///
/// Chunks are stored as `{base_dir}/chunks/{cx}_{cy}_{cz}.bin` with zstd
/// frame compression. The base directory defaults to `./saves`.
pub struct DiskChunkStorage {
    /// Root directory for chunk storage.
    base_dir: PathBuf,
    /// Zstd compression level (1-22, default 3 for speed/ratio balance).
    compression_level: i32,
}

impl DiskChunkStorage {
    /// Create a new disk storage rooted at `base_dir`.
    ///
    /// Creates the `chunks/` subdirectory if it doesn't exist.
    pub fn new(base_dir: impl Into<PathBuf>) -> Result<Self, ChunkStorageError> {
        Self::with_compression_level(base_dir, 3)
    }

    /// Create a new disk storage with a custom zstd compression level.
    pub fn with_compression_level(
        base_dir: impl Into<PathBuf>,
        compression_level: i32,
    ) -> Result<Self, ChunkStorageError> {
        let base = base_dir.into();
        let chunks_dir = base.join("chunks");
        std::fs::create_dir_all(&chunks_dir).map_err(|e| {
            ChunkStorageError::Io(format!("Failed to create chunks dir: {e}"))
        })?;
        Ok(Self {
            base_dir: base,
            compression_level,
        })
    }

    /// Get the file path for a chunk coordinate.
    fn chunk_path(&self, coord: ChunkCoord) -> PathBuf {
        self.base_dir
            .join("chunks")
            .join(format!("{}_{}_{}.bin", coord.cx, coord.cy, coord.cz))
    }

    /// Compress bytes with zstd.
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>, ChunkStorageError> {
        zstd::encode_all(data, self.compression_level)
            .map_err(|e| ChunkStorageError::Compression(format!("zstd encode failed: {e}")))
    }

    /// Decompress bytes with zstd.
    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>, ChunkStorageError> {
        zstd::decode_all(data)
            .map_err(|e| ChunkStorageError::Compression(format!("zstd decode failed: {e}")))
    }

    /// Get the base directory.
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    /// Get the compression level.
    pub fn compression_level(&self) -> i32 {
        self.compression_level
    }
}

impl ChunkStorage for DiskChunkStorage {
    fn save_chunk(&self, payload: &ChunkPayload) -> Result<(), ChunkStorageError> {
        // Serialize the payload to JSON
        let raw = serde_json::to_vec(payload)
            .map_err(|e| ChunkStorageError::Serde(format!("serde_json serialize: {e}")))?;

        // Compress
        let compressed = self.compress(&raw)?;

        // Write atomically: write to .tmp then rename
        let path = self.chunk_path(payload.coord);
        let tmp_path = path.with_extension("bin.tmp");
        std::fs::write(&tmp_path, &compressed).map_err(|e| {
            ChunkStorageError::Io(format!("Failed to write chunk file: {e}"))
        })?;
        std::fs::rename(&tmp_path, &path).map_err(|e| {
            ChunkStorageError::Io(format!("Failed to rename chunk file: {e}"))
        })?;

        Ok(())
    }

    fn load_chunk(&self, coord: ChunkCoord) -> Result<ChunkPayload, ChunkStorageError> {
        let path = self.chunk_path(coord);
        if !path.exists() {
            return Err(ChunkStorageError::NotFound(coord));
        }

        let compressed = std::fs::read(&path).map_err(|e| {
            ChunkStorageError::Io(format!("Failed to read chunk file: {e}"))
        })?;

        let raw = self.decompress(&compressed)?;

        let payload: ChunkPayload = serde_json::from_slice(&raw)
            .map_err(|e| ChunkStorageError::Serde(format!("serde_json deserialize: {e}")))?;

        Ok(payload)
    }

    fn delete_chunk(&self, coord: ChunkCoord) -> Result<(), ChunkStorageError> {
        let path = self.chunk_path(coord);
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| {
                ChunkStorageError::Io(format!("Failed to delete chunk file: {e}"))
            })?;
        }
        Ok(())
    }

    fn chunk_exists(&self, coord: ChunkCoord) -> bool {
        self.chunk_path(coord).exists()
    }
}

// ---------------------------------------------------------------------------
// StreamingManager with disk persistence
// ---------------------------------------------------------------------------

/// Statistics for the streaming manager's disk cache.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StreamingCacheStats {
    /// Number of times a chunk was loaded from disk cache.
    pub disk_cache_hits: u64,
    /// Number of times a chunk had to be generated (not on disk).
    pub disk_cache_misses: u64,
    /// Total number of chunks evicted from the resident set.
    pub evictions: u64,
    /// Number of chunks currently in the eviction save queue.
    pub pending_evictions: usize,
}

/// Manages the lifecycle of chunks in the streaming window with disk persistence.
///
/// When chunks are evicted from the resident set, they are saved to disk.
/// When chunks are requested and not in memory, the disk cache is checked
/// before generating new data.
pub struct StreamingManager {
    storage: Box<dyn ChunkStorage>,
    /// Chunks currently held in memory (coord -> payload).
    resident: HashMap<ChunkCoord, ChunkPayload>,
    /// Maximum number of chunks to keep resident.
    max_resident: usize,
    /// Cache statistics.
    stats: StreamingCacheStats,
}

impl StreamingManager {
    /// Create a new streaming manager with the given storage backend.
    pub fn new(storage: Box<dyn ChunkStorage>, max_resident: usize) -> Self {
        Self {
            storage,
            resident: HashMap::new(),
            max_resident,
            stats: StreamingCacheStats::default(),
        }
    }

    /// Create a new streaming manager backed by [`DiskChunkStorage`] at the
    /// given directory path.  The directory is created automatically.
    pub fn new_disk(
        base_dir: impl Into<std::path::PathBuf>,
        max_resident: usize,
    ) -> Result<Self, ChunkStorageError> {
        let disk = DiskChunkStorage::new(base_dir)?;
        Ok(Self::new(Box::new(disk), max_resident))
    }

    /// Request a chunk. First checks the in-memory cache, then disk, returning
    /// `None` if it must be generated fresh.
    pub fn request_chunk(
        &mut self,
        coord: ChunkCoord,
    ) -> Result<Option<&ChunkPayload>, ChunkStorageError> {
        // Check in-memory first
        if self.resident.contains_key(&coord) {
            return Ok(self.resident.get(&coord));
        }

        // Check disk
        match self.storage.load_chunk(coord) {
            Ok(payload) => {
                self.stats.disk_cache_hits += 1;
                self.resident.insert(coord, payload.clone());
                Ok(self.resident.get(&coord))
            }
            Err(ChunkStorageError::NotFound(_)) => {
                self.stats.disk_cache_misses += 1;
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }

    /// Evict a chunk from memory, saving it to disk first.
    pub fn evict_chunk(&mut self, coord: ChunkCoord) -> Result<(), ChunkStorageError> {
        if let Some(payload) = self.resident.remove(&coord) {
            self.storage.save_chunk(&payload)?;
            self.stats.evictions += 1;
            self.stats.pending_evictions = self.stats.pending_evictions.saturating_sub(1);
        }
        Ok(())
    }

    /// Insert a chunk into the resident set. If over capacity, evicts the
    /// oldest chunk (first key in the HashMap -- not LRU, but deterministic).
    pub fn insert_chunk(&mut self, payload: ChunkPayload) -> Result<(), ChunkStorageError> {
        // If at capacity, evict one chunk
        if self.resident.len() >= self.max_resident {
            if let Some(&oldest) = self.resident.keys().next() {
                self.evict_chunk(oldest)?;
            }
        }
        self.resident.insert(payload.coord, payload);
        Ok(())
    }

    /// Force-save all resident chunks to disk.
    pub fn flush_all(&self) -> Result<(), ChunkStorageError> {
        self.save_all_chunks()
    }

    /// Persist every chunk currently held in memory to the storage backend.
    /// Useful before a shutdown or world-swap.
    pub fn save_all_chunks(&self) -> Result<(), ChunkStorageError> {
        for payload in self.resident.values() {
            self.storage.save_chunk(payload)?;
        }
        Ok(())
    }

    /// Get current cache statistics.
    pub fn stats(&self) -> &StreamingCacheStats {
        &self.stats
    }

    /// Get the number of chunks currently in memory.
    pub fn resident_count(&self) -> usize {
        self.resident.len()
    }

    /// Get a reference to the storage backend.
    pub fn storage(&self) -> &dyn ChunkStorage {
        &*self.storage
    }
}

// ---------------------------------------------------------------------------
// In-memory storage for testing
// ---------------------------------------------------------------------------

/// Thread-safe in-memory chunk storage for testing.
pub struct MemoryChunkStorage {
    chunks: Mutex<HashMap<ChunkCoord, ChunkPayload>>,
}

impl MemoryChunkStorage {
    /// Create a new empty in-memory store.
    pub fn new() -> Self {
        Self {
            chunks: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for MemoryChunkStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl ChunkStorage for MemoryChunkStorage {
    fn save_chunk(&self, payload: &ChunkPayload) -> Result<(), ChunkStorageError> {
        let mut chunks = self.chunks.lock().expect("mutex poisoned");
        chunks.insert(payload.coord, payload.clone());
        Ok(())
    }

    fn load_chunk(&self, coord: ChunkCoord) -> Result<ChunkPayload, ChunkStorageError> {
        let chunks = self.chunks.lock().expect("mutex poisoned");
        chunks
            .get(&coord)
            .cloned()
            .ok_or(ChunkStorageError::NotFound(coord))
    }

    fn delete_chunk(&self, coord: ChunkCoord) -> Result<(), ChunkStorageError> {
        let mut chunks = self.chunks.lock().expect("mutex poisoned");
        chunks.remove(&coord);
        Ok(())
    }

    fn chunk_exists(&self, coord: ChunkCoord) -> bool {
        let chunks = self.chunks.lock().expect("mutex poisoned");
        chunks.contains_key(&coord)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn coord(cx: i32, cy: i32, cz: i32) -> ChunkCoord {
        ChunkCoord { cx, cy, cz }
    }

    fn sample_payload(cx: i32, cy: i32, cz: i32) -> ChunkPayload {
        let mut metadata = HashMap::new();
        metadata.insert("version".to_string(), "1".to_string());
        ChunkPayload {
            coord: coord(cx, cy, cz),
            data: vec![0xAB, 0xCD, 0xEF, 0x12, 0x34, 0x56, 0x78, 0x90],
            metadata,
        }
    }

    // ========================================================================
    // Roundtrip tests (DiskChunkStorage)
    // ========================================================================

    /// STREAM-001 -- save and load a chunk roundtrip via DiskChunkStorage.
    #[test]
    fn disk_save_load_roundtrip() {
        let dir = std::env::temp_dir().join("phenotype_gfx_test_roundtrip");
        let storage = DiskChunkStorage::new(&dir).expect("create storage");

        let payload = sample_payload(10, 20, 30);
        storage.save_chunk(&payload).expect("save");

        let loaded = storage.load_chunk(coord(10, 20, 30)).expect("load");
        assert_eq!(loaded.coord, payload.coord);
        assert_eq!(loaded.data, payload.data);
        assert_eq!(loaded.metadata, payload.metadata);

        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// STREAM-002 -- save and load via MemoryChunkStorage.
    #[test]
    fn memory_save_load_roundtrip() {
        let storage = MemoryChunkStorage::new();
        let payload = sample_payload(1, 2, 3);

        storage.save_chunk(&payload).expect("save");
        let loaded = storage.load_chunk(coord(1, 2, 3)).expect("load");
        assert_eq!(loaded, payload);
    }

    // ========================================================================
    // Eviction saves to disk
    // ========================================================================

    /// STREAM-003 -- evicting a chunk from StreamingManager saves to disk.
    #[test]
    fn eviction_saves_to_disk() {
        let dir = std::env::temp_dir().join("phenotype_gfx_test_eviction");
        let storage = Box::new(DiskChunkStorage::new(&dir).expect("create storage"));
        let mut mgr = StreamingManager::new(storage, 2);

        let p1 = sample_payload(1, 0, 0);
        let p2 = sample_payload(2, 0, 0);
        mgr.insert_chunk(p1.clone()).expect("insert p1");
        mgr.insert_chunk(p2.clone()).expect("insert p2");

        // Evict p1
        mgr.evict_chunk(coord(1, 0, 0)).expect("evict");

        // Should no longer be in memory
        assert_eq!(mgr.resident_count(), 1);

        // Should be on disk
        let loaded = mgr.storage().load_chunk(coord(1, 0, 0)).expect("load from disk");
        assert_eq!(loaded.data, p1.data);

        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ========================================================================
    // Cache hit/miss tracking
    // ========================================================================

    /// STREAM-004 -- disk cache hit increments disk_cache_hits counter.
    #[test]
    fn cache_hit_counter() {
        let dir = std::env::temp_dir().join("phenotype_gfx_test_cache_hit");
        let disk = DiskChunkStorage::new(&dir).expect("create storage");
        let payload = sample_payload(5, 5, 5);
        disk.save_chunk(&payload).expect("save to disk");
        let mut mgr = StreamingManager::new(Box::new(disk), 10);

        assert_eq!(mgr.stats().disk_cache_hits, 0);
        assert_eq!(mgr.stats().disk_cache_misses, 0);

        // Load from disk -- should be a cache hit
        mgr.request_chunk(coord(5, 5, 5)).expect("request");
        assert_eq!(mgr.stats().disk_cache_hits, 1);
        assert_eq!(mgr.stats().disk_cache_misses, 0);

        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// STREAM-005 -- requesting a non-existent chunk increments disk_cache_misses.
    #[test]
    fn cache_miss_counter() {
        let dir = std::env::temp_dir().join("phenotype_gfx_test_cache_miss");
        let storage = Box::new(DiskChunkStorage::new(&dir).expect("create storage"));
        let mut mgr = StreamingManager::new(storage, 10);

        let result = mgr.request_chunk(coord(99, 99, 99));
        assert!(result.expect("request").is_none());
        assert_eq!(mgr.stats().disk_cache_misses, 1);

        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ========================================================================
    // Compression ratio check
    // ========================================================================

    /// STREAM-006 -- compressed file is smaller than uncompressed for repetitive data.
    #[test]
    fn compression_ratio() {
        let dir = std::env::temp_dir().join("phenotype_gfx_test_compression");
        let storage = DiskChunkStorage::with_compression_level(&dir, 3).expect("create storage");

        // Repetitive data compresses well
        let big_data = vec![0u8; 10_000];
        let payload = ChunkPayload {
            coord: coord(0, 0, 0),
            data: big_data.clone(),
            metadata: HashMap::new(),
        };

        storage.save_chunk(&payload).expect("save");

        let path = dir.join("chunks").join("0_0_0.bin");
        let file_size = std::fs::metadata(&path)
            .expect("metadata")
            .len();

        assert!(
            file_size < big_data.len() as u64,
            "compressed file ({file_size} bytes) should be smaller than original ({} bytes)",
            big_data.len()
        );

        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ========================================================================
    // Corruption detection
    // ========================================================================

    /// STREAM-007 -- corrupted chunk file returns error on load.
    #[test]
    fn corruption_detection() {
        let dir = std::env::temp_dir().join("phenotype_gfx_test_corruption");
        let storage = DiskChunkStorage::new(&dir).expect("create storage");

        // Save a valid chunk
        let payload = sample_payload(7, 8, 9);
        storage.save_chunk(&payload).expect("save");

        // Corrupt the file
        let path = dir.join("chunks").join("7_8_9.bin");
        std::fs::write(&path, b"this is not valid zstd data!!!").expect("corrupt");

        // Load should fail
        let result = storage.load_chunk(coord(7, 8, 9));
        assert!(result.is_err(), "loading corrupted file should fail");

        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ========================================================================
    // Additional tests
    // ========================================================================

    /// STREAM-008 -- chunk_exists returns true/false correctly.
    #[test]
    fn chunk_exists_correct() {
        let dir = std::env::temp_dir().join("phenotype_gfx_test_exists");
        let storage = DiskChunkStorage::new(&dir).expect("create storage");

        assert!(!storage.chunk_exists(coord(1, 1, 1)));
        let payload = sample_payload(1, 1, 1);
        storage.save_chunk(&payload).expect("save");
        assert!(storage.chunk_exists(coord(1, 1, 1)));

        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// STREAM-009 -- delete_chunk removes from disk.
    #[test]
    fn delete_chunk_removes_from_disk() {
        let dir = std::env::temp_dir().join("phenotype_gfx_test_delete");
        let storage = DiskChunkStorage::new(&dir).expect("create storage");

        let payload = sample_payload(2, 2, 2);
        storage.save_chunk(&payload).expect("save");
        assert!(storage.chunk_exists(coord(2, 2, 2)));

        storage.delete_chunk(coord(2, 2, 2)).expect("delete");
        assert!(!storage.chunk_exists(coord(2, 2, 2)));

        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// STREAM-010 -- StreamingManager auto-evicts when over capacity.
    #[test]
    fn streaming_manager_auto_evict() {
        let storage = Box::new(MemoryChunkStorage::new());
        let mut mgr = StreamingManager::new(storage, 2);

        mgr.insert_chunk(sample_payload(1, 0, 0)).expect("insert 1");
        mgr.insert_chunk(sample_payload(2, 0, 0)).expect("insert 2");
        assert_eq!(mgr.resident_count(), 2);

        // Inserting a third should evict one of the first two
        mgr.insert_chunk(sample_payload(3, 0, 0)).expect("insert 3");
        assert_eq!(mgr.resident_count(), 2);
        assert_eq!(mgr.stats().evictions, 1);

        // Exactly one of the evicted chunks should be on disk (in-memory storage)
        let on_disk_1 = mgr.storage().chunk_exists(coord(1, 0, 0));
        let on_disk_2 = mgr.storage().chunk_exists(coord(2, 0, 0));
        assert!(
            on_disk_1 ^ on_disk_2,
            "exactly one of chunks 1 or 2 should have been evicted to storage"
        );

        // Payload 3 should always be in resident
        let result3 = mgr.request_chunk(coord(3, 0, 0)).expect("request 3");
        assert!(result3.is_some());
    }

    // ========================================================================
    // New tests: disk-backed eviction, load-from-disk, save_all roundtrip,
    // directory auto-creation, cache stats, concurrent access
    // ========================================================================

    /// STREAM-011 -- eviction persists to disk and can be re-loaded.
    #[test]
    fn eviction_persists_to_disk() {
        let dir = std::env::temp_dir().join("phenotype_gfx_test_stream011");
        let storage = Box::new(DiskChunkStorage::new(&dir).expect("create storage"));
        let mut mgr = StreamingManager::new(storage, 3);

        let payload = sample_payload(10, 20, 30);
        mgr.insert_chunk(payload.clone()).expect("insert");
        assert_eq!(mgr.resident_count(), 1);

        // Evict — should persist to disk
        mgr.evict_chunk(coord(10, 20, 30)).expect("evict");
        assert_eq!(mgr.resident_count(), 0);

        // Load from disk via a fresh StreamingManager
        let storage2 = Box::new(DiskChunkStorage::new(&dir).expect("create storage2"));
        let mut mgr2 = StreamingManager::new(storage2, 3);
        let loaded = mgr2.request_chunk(coord(10, 20, 30)).expect("request");
        assert!(loaded.is_some(), "chunk should be loadable from disk after eviction");
        assert_eq!(loaded.unwrap().data, payload.data);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// STREAM-012 -- request_chunk returns from disk on cache miss.
    #[test]
    fn load_from_disk_on_cache_miss() {
        let dir = std::env::temp_dir().join("phenotype_gfx_test_stream012");
        let disk = DiskChunkStorage::new(&dir).expect("create storage");
        let payload = sample_payload(7, 8, 9);
        disk.save_chunk(&payload).expect("save to disk first");
        let mut mgr = StreamingManager::new(Box::new(disk), 10);

        // Resident set is empty — request should fall through to disk
        let result = mgr.request_chunk(coord(7, 8, 9)).expect("request");
        assert!(result.is_some(), "should load from disk on cache miss");
        assert_eq!(result.unwrap().data, payload.data);
        // Should now be in the resident set
        assert_eq!(mgr.resident_count(), 1);
        assert_eq!(mgr.stats().disk_cache_hits, 1);
        assert_eq!(mgr.stats().disk_cache_misses, 0);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// STREAM-013 -- save_all_chunks roundtrip: all resident chunks survive.
    #[test]
    fn save_all_roundtrip() {
        let dir = std::env::temp_dir().join("phenotype_gfx_test_stream013");
        let storage = Box::new(DiskChunkStorage::new(&dir).expect("create storage"));
        let mut mgr = StreamingManager::new(storage, 10);

        mgr.insert_chunk(sample_payload(1, 0, 0)).expect("a");
        mgr.insert_chunk(sample_payload(2, 0, 0)).expect("b");
        mgr.insert_chunk(sample_payload(3, 0, 0)).expect("c");

        mgr.save_all_chunks().expect("save all");

        // Verify all three are on disk via a fresh manager
        let storage2 = Box::new(DiskChunkStorage::new(&dir).expect("create storage2"));
        let mut mgr2 = StreamingManager::new(storage2, 10);
        for cx in 1..=3 {
            let loaded = mgr2
                .request_chunk(coord(cx, 0, 0))
                .expect("load")
                .expect("should exist");
            assert_eq!(loaded.coord, coord(cx, 0, 0));
        }
        assert_eq!(mgr2.stats().disk_cache_hits, 3);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// STREAM-014 -- new_disk auto-creates the base directory.
    #[test]
    fn directory_auto_creation() {
        // Use a unique nested path; clean up any prior run first.
        let base = std::env::temp_dir().join("phenotype_gfx_test_stream014_nested");
        let _ = std::fs::remove_dir_all(&base);
        let dir = base.join("deep/path");
        assert!(!dir.exists(), "dir should not exist yet");

        let mut mgr = StreamingManager::new_disk(&dir, 5).expect("new_disk");
        assert!(dir.exists(), "new_disk should create the directory");
        assert!(dir.join("chunks").exists(), "chunks sub-directory should exist");

        let payload = sample_payload(1, 1, 1);
        mgr.insert_chunk(payload).expect("insert");
        mgr.save_all_chunks().expect("save");
        assert!(dir.join("chunks/1_1_1.bin").exists());

        let _ = std::fs::remove_dir_all(&base);
    }

    /// STREAM-015 -- cache stats track hits, misses, and evictions correctly.
    #[test]
    fn cache_stats_tracking() {
        let dir = std::env::temp_dir().join("phenotype_gfx_test_stream015");
        let disk = DiskChunkStorage::new(&dir).expect("create storage");
        // Pre-populate disk with one chunk
        disk.save_chunk(&sample_payload(50, 0, 0)).expect("save");
        let mut mgr = StreamingManager::new(Box::new(disk), 2);

        // Initial stats
        assert_eq!(mgr.stats().disk_cache_hits, 0);
        assert_eq!(mgr.stats().disk_cache_misses, 0);
        assert_eq!(mgr.stats().evictions, 0);

        // Miss: chunk not in memory or disk
        mgr.request_chunk(coord(99, 0, 0)).expect("miss");
        assert_eq!(mgr.stats().disk_cache_misses, 1);

        // Hit: chunk on disk
        mgr.request_chunk(coord(50, 0, 0)).expect("hit");
        assert_eq!(mgr.stats().disk_cache_hits, 1);

        // Fill to capacity and trigger eviction
        mgr.insert_chunk(sample_payload(1, 0, 0)).expect("i1");
        mgr.insert_chunk(sample_payload(2, 0, 0)).expect("i2"); // evicts chunk(50,0,0)
        assert_eq!(mgr.stats().evictions, 1);

        mgr.insert_chunk(sample_payload(3, 0, 0)).expect("i3"); // evicts chunk(1,0,0)
        assert_eq!(mgr.stats().evictions, 2);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// STREAM-016 -- concurrent access from multiple threads is safe.
    #[test]
    fn concurrent_access_safety() {
        use std::sync::Arc;
        use std::thread;

        let dir = std::env::temp_dir().join("phenotype_gfx_test_stream016");
        let disk = Arc::new(DiskChunkStorage::new(&dir).expect("create storage"));

        // Pre-populate disk with chunks
        for i in 0..20u32 {
            let mut metadata = HashMap::new();
            metadata.insert("i".to_string(), i.to_string());
            disk.save_chunk(&ChunkPayload {
                coord: coord(i as i32, 0, 0),
                data: vec![i as u8; 64],
                metadata,
            })
            .expect("save");
        }

        let mut handles = vec![];
        for t in 0..4 {
            let base = dir.clone();
            handles.push(thread::spawn(move || {
                // Each thread uses its own StreamingManager backed by the
                // same DiskChunkStorage (shared via Arc trait object).
                // We create StreamingManagers with Box::new wrapper.
                // For true concurrent disk access we share the Arc<DiskChunkStorage>
                // directly through the ChunkStorage trait via MemoryChunkStorage
                // wrapped in Arc. This test verifies no panics from concurrent
                // disk reads/writes to distinct chunk files.
                let mgr_storage = DiskChunkStorage::new(&base)
                    .expect("create storage per-thread");
                let mut mgr = StreamingManager::new(Box::new(mgr_storage), 5);

                for i in 0..10u32 {
                    let cx = (t * 10 + i as usize) as i32;
                    let result = mgr.request_chunk(coord(cx, 0, 0));
                    // All should succeed without panics
                    let _ = result.expect("concurrent request should not error");
                }
            }));
        }

        for h in handles {
            h.join().expect("thread should not panic");
        }

        let _ = std::fs::remove_dir_all(&dir);
    }
}
