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

use crate::lod_priority::{self, CameraVelocity, PriorityScore};
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
                write!(
                    f,
                    "Chunk not found at ({}, {}, {})",
                    coord.cx, coord.cy, coord.cz
                )
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
        std::fs::create_dir_all(&chunks_dir)
            .map_err(|e| ChunkStorageError::Io(format!("Failed to create chunks dir: {e}")))?;
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
        let started = std::time::Instant::now();
        crate::gfx_trace!(
            "streaming_io: save_chunk start chunk_id={:?} bytes={}",
            payload.coord,
            payload.data.len()
        );
        crate::gfx_debug!(
            "streaming_io: save chunk {:?} bytes={}",
            payload.coord,
            payload.data.len()
        );
        // Serialize the payload to JSON
        let raw = serde_json::to_vec(payload)
            .map_err(|e| ChunkStorageError::Serde(format!("serde_json serialize: {e}")))?;

        // Compress
        let compressed = self.compress(&raw)?;

        // Write atomically: write to .tmp then rename
        let path = self.chunk_path(payload.coord);
        let tmp_path = path.with_extension("bin.tmp");
        std::fs::write(&tmp_path, &compressed)
            .map_err(|e| ChunkStorageError::Io(format!("Failed to write chunk file: {e}")))?;
        std::fs::rename(&tmp_path, &path)
            .map_err(|e| ChunkStorageError::Io(format!("Failed to rename chunk file: {e}")))?;

        crate::gfx_trace!(
            "streaming_io: save_chunk done chunk_id={:?} bytes_in={} compressed_bytes={} elapsed_ms={:.3}",
            payload.coord,
            payload.data.len(),
            compressed.len(),
            started.elapsed().as_secs_f64() * 1000.0
        );

        Ok(())
    }

    fn load_chunk(&self, coord: ChunkCoord) -> Result<ChunkPayload, ChunkStorageError> {
        let started = std::time::Instant::now();
        crate::gfx_trace!(
            "streaming_io: load_chunk start chunk_id={:?}",
            coord
        );
        let path = self.chunk_path(coord);
        if !path.exists() {
            crate::gfx_debug!(
                "streaming_io: cache miss (no file) chunk_id={:?}",
                coord
            );
            return Err(ChunkStorageError::NotFound(coord));
        }

        let compressed = std::fs::read(&path)
            .map_err(|e| ChunkStorageError::Io(format!("Failed to read chunk file: {e}")))?;

        let raw = self.decompress(&compressed)?;

        let payload: ChunkPayload = serde_json::from_slice(&raw)
            .map_err(|e| ChunkStorageError::Serde(format!("serde_json deserialize: {e}")))?;

        crate::gfx_debug!(
            "streaming_io: load chunk {:?} bytes={}",
            coord,
            payload.data.len()
        );
        crate::gfx_trace!(
            "streaming_io: load_chunk done chunk_id={:?} bytes={} elapsed_ms={:.3}",
            coord,
            payload.data.len(),
            started.elapsed().as_secs_f64() * 1000.0
        );

        Ok(payload)
    }

    fn delete_chunk(&self, coord: ChunkCoord) -> Result<(), ChunkStorageError> {
        let path = self.chunk_path(coord);
        if path.exists() {
            std::fs::remove_file(&path)
                .map_err(|e| ChunkStorageError::Io(format!("Failed to delete chunk file: {e}")))?;
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
    /// Number of chunks loaded via prefetch this frame.
    pub prefetch_hits: u64,
}

/// Manages the lifecycle of chunks in the streaming window with disk persistence.
///
/// When chunks are evicted from the resident set, they are saved to disk.
/// When chunks are requested and not in memory, the disk cache is checked
/// before generating new data.
///
/// Eviction is priority-based: chunks closest to the camera, at the highest
/// LOD detail, and most recently accessed are evicted LAST. Chunks far away,
/// at low LOD detail, and not recently accessed are evicted FIRST.
pub struct StreamingManager {
    storage: Box<dyn ChunkStorage>,
    /// Chunks currently held in memory (coord -> payload).
    resident: HashMap<ChunkCoord, ChunkPayload>,
    /// Maximum number of chunks to keep resident.
    max_resident: usize,
    /// Cache statistics.
    stats: StreamingCacheStats,
    /// Per-chunk LOD level (higher = coarser detail).
    lod_levels: HashMap<ChunkCoord, u8>,
    /// Per-chunk last-access tick (for recency scoring).
    last_access: HashMap<ChunkCoord, u32>,
    /// Current tick counter (incremented each frame).
    current_tick: u32,
    /// Camera anchor chunk coordinate. `None` if no camera has been set.
    camera_anchor: Option<ChunkCoord>,
    /// Camera velocity for predictive prefetch.
    camera_velocity: CameraVelocity,
    /// Vertical weight for ring-distance metric.
    vy_weight: u8,
    /// Maximum LOD level in the system.
    max_lod_level: u8,
    /// Recency decay in ticks (accesses older than this score 0 recency).
    recency_decay_ticks: u32,
    /// Maximum number of chunks to prefetch per frame.
    prefetch_budget: usize,
    /// Mesh ring from WindowPolicy (chunks within this ring are always meshed).
    mesh_ring: u8,
    /// Prefetch ring extension past mesh_ring.
    prefetch_ring: u8,
}

impl StreamingManager {
    /// Create a new streaming manager with the given storage backend.
    ///
    /// Uses default LOD parameters (max_lod_level=4, vy_weight=2, etc.).
    pub fn new(storage: Box<dyn ChunkStorage>, max_resident: usize) -> Self {
        Self {
            storage,
            resident: HashMap::new(),
            max_resident,
            stats: StreamingCacheStats::default(),
            lod_levels: HashMap::new(),
            last_access: HashMap::new(),
            current_tick: 0,
            camera_anchor: None,
            camera_velocity: CameraVelocity::default(),
            vy_weight: 2,
            max_lod_level: 4,
            recency_decay_ticks: 60,
            prefetch_budget: lod_priority::DEFAULT_PREFETCH_BUDGET,
            mesh_ring: 1,
            prefetch_ring: 3,
        }
    }

    /// Create a new streaming manager with full LOD priority configuration.
    #[allow(clippy::too_many_arguments)]
    pub fn with_lod_config(
        storage: Box<dyn ChunkStorage>,
        max_resident: usize,
        vy_weight: u8,
        max_lod_level: u8,
        recency_decay_ticks: u32,
        prefetch_budget: usize,
        mesh_ring: u8,
        prefetch_ring: u8,
    ) -> Self {
        Self {
            storage,
            resident: HashMap::new(),
            max_resident,
            stats: StreamingCacheStats::default(),
            lod_levels: HashMap::new(),
            last_access: HashMap::new(),
            current_tick: 0,
            camera_anchor: None,
            camera_velocity: CameraVelocity::default(),
            vy_weight,
            max_lod_level,
            recency_decay_ticks,
            prefetch_budget,
            mesh_ring,
            prefetch_ring,
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

    // --------------------------------------------------------------------
    // Camera & LOD state management
    // --------------------------------------------------------------------

    /// Set the camera anchor chunk coordinate.
    pub fn set_camera_anchor(&mut self, anchor: ChunkCoord) {
        self.camera_anchor = Some(anchor);
    }

    /// Get the current camera anchor, if set.
    pub fn camera_anchor(&self) -> Option<ChunkCoord> {
        self.camera_anchor
    }

    /// Set the camera velocity for predictive prefetch.
    pub fn set_camera_velocity(&mut self, velocity: CameraVelocity) {
        self.camera_velocity = velocity;
    }

    /// Get the current camera velocity.
    pub fn camera_velocity(&self) -> CameraVelocity {
        self.camera_velocity
    }

    /// Advance the tick counter. Call once per frame.
    pub fn advance_tick(&mut self) {
        self.current_tick = self.current_tick.saturating_add(1);
    }

    /// Get the current tick.
    pub fn current_tick(&self) -> u32 {
        self.current_tick
    }

    /// Set the LOD level for a specific chunk.
    pub fn set_lod_level(&mut self, coord: ChunkCoord, lod_level: u8) {
        self.lod_levels.insert(coord, lod_level);
    }

    /// Get the LOD level for a chunk (defaults to max_lod_level if unknown).
    pub fn get_lod_level(&self, coord: ChunkCoord) -> u8 {
        self.lod_levels
            .get(&coord)
            .copied()
            .unwrap_or(self.max_lod_level)
    }

    /// Compute the priority score for a resident chunk.
    fn chunk_priority(&self, coord: ChunkCoord) -> PriorityScore {
        let anchor = self.camera_anchor.unwrap_or(coord);
        let lod = self.get_lod_level(coord);
        let last_access = self.last_access.get(&coord).copied().unwrap_or(0);
        lod_priority::compute_priority(
            coord,
            anchor,
            self.vy_weight,
            lod,
            self.max_lod_level,
            last_access,
            self.current_tick,
            self.recency_decay_ticks,
        )
    }

    /// Find the lowest-priority resident chunk for eviction.
    fn lowest_priority_coord(&self) -> Option<ChunkCoord> {
        self.resident
            .keys()
            .min_by(|&a, &b| {
                let pa = self.chunk_priority(*a);
                let pb = self.chunk_priority(*b);
                pa.weighted()
                    .partial_cmp(&pb.weighted())
                    .unwrap_or(core::cmp::Ordering::Equal)
            })
            .copied()
    }

    // --------------------------------------------------------------------
    // Core chunk lifecycle
    // --------------------------------------------------------------------

    /// Request a chunk. First checks the in-memory cache, then disk, returning
    /// `None` if it must be generated fresh.
    pub fn request_chunk(
        &mut self,
        coord: ChunkCoord,
    ) -> Result<Option<&ChunkPayload>, ChunkStorageError> {
        // Check in-memory first — boost recency on hit
        if self.resident.contains_key(&coord) {
            self.last_access.insert(coord, self.current_tick);
            return Ok(self.resident.get(&coord));
        }

        // Check disk
        match self.storage.load_chunk(coord) {
            Ok(payload) => {
                self.stats.disk_cache_hits += 1;
                // Disk-load priority boost: mark as recently accessed
                self.last_access.insert(coord, self.current_tick);
                self.resident.insert(coord, payload.clone());
                Ok(self.resident.get(&coord))
            }
            Err(ChunkStorageError::NotFound(_)) => {
                self.stats.disk_cache_misses += 1;
                crate::gfx_debug!(
                    "streaming_io: cache miss (not on disk) chunk_id={:?}",
                    coord
                );
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }

    /// Evict a chunk from memory, saving it to disk first.
    pub fn evict_chunk(&mut self, coord: ChunkCoord) -> Result<(), ChunkStorageError> {
        let started = std::time::Instant::now();
        crate::gfx_trace!(
            "streaming_io: evict_chunk start chunk_id={:?}",
            coord
        );
        if let Some(payload) = self.resident.remove(&coord) {
            self.storage.save_chunk(&payload)?;
            self.stats.evictions += 1;
            self.stats.pending_evictions = self.stats.pending_evictions.saturating_sub(1);
            self.lod_levels.remove(&coord);
            self.last_access.remove(&coord);
            crate::gfx_debug!(
                "streaming_io: eviction triggered chunk_id={:?} bytes={} elapsed_ms={:.3}",
                coord,
                payload.data.len(),
                started.elapsed().as_secs_f64() * 1000.0
            );
        }
        Ok(())
    }

    /// Insert a chunk into the resident set with a given LOD level.
    ///
    /// If over capacity, the **lowest-priority** chunk is evicted first
    /// (priority-based eviction, not FIFO).
    pub fn insert_chunk_with_lod(
        &mut self,
        payload: ChunkPayload,
        lod_level: u8,
    ) -> Result<(), ChunkStorageError> {
        self.lod_levels.insert(payload.coord, lod_level);
        self.last_access.insert(payload.coord, self.current_tick);
        self.insert_chunk(payload)
    }

    /// Insert a chunk into the resident set. If over capacity, evicts the
    /// lowest-priority chunk (distance + LOD + recency), not FIFO.
    pub fn insert_chunk(&mut self, payload: ChunkPayload) -> Result<(), ChunkStorageError> {
        // If at capacity, evict the lowest-priority chunk
        if self.resident.len() >= self.max_resident {
            if let Some(victim) = self.lowest_priority_coord() {
                crate::gfx_debug!(
                    "streaming_io: eviction triggered by insert chunk_id={:?} victim={:?}",
                    payload.coord,
                    victim
                );
                self.evict_chunk(victim)?;
            }
        }
        self.last_access
            .entry(payload.coord)
            .or_insert(self.current_tick);
        self.resident.insert(payload.coord, payload);
        Ok(())
    }

    // --------------------------------------------------------------------
    // Predictive prefetching
    // --------------------------------------------------------------------

    /// Predict and pre-load chunks likely needed next frame.
    ///
    /// Uses the current camera anchor and velocity to forecast which chunks
    /// will enter the streaming window. Loads up to `prefetch_budget` chunks
    /// from disk into the resident cache.
    ///
    /// Returns the number of chunks actually prefetched.
    pub fn prefetch_next_frame(&mut self) -> Result<usize, ChunkStorageError> {
        let anchor = match self.camera_anchor {
            Some(a) => a,
            None => return Ok(0),
        };

        let predicted = lod_priority::predict_chunks(
            anchor,
            self.camera_velocity,
            self.vy_weight,
            self.mesh_ring,
            self.prefetch_ring,
            self.prefetch_budget,
        );

        let mut prefetched = 0usize;
        for coord in predicted {
            if self.resident.contains_key(&coord) {
                continue; // already resident
            }
            if prefetched >= self.prefetch_budget {
                break;
            }
            // Load from disk if available
            match self.storage.load_chunk(coord) {
                Ok(payload) => {
                    // Evict lowest-priority if at capacity
                    if self.resident.len() >= self.max_resident {
                        if let Some(victim) = self.lowest_priority_coord() {
                            self.evict_chunk(victim)?;
                        }
                    }
                    self.last_access.insert(coord, self.current_tick);
                    self.resident.insert(coord, payload);
                    self.stats.prefetch_hits += 1;
                    prefetched += 1;
                }
                Err(ChunkStorageError::NotFound(_)) => {
                    // Not on disk — skip (caller should generate)
                }
                Err(e) => return Err(e),
            }
        }

        Ok(prefetched)
    }

    // --------------------------------------------------------------------
    // Flush / persistence
    // --------------------------------------------------------------------

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

    /// Check if a chunk is currently held in the resident (in-memory) set.
    pub fn is_resident(&self, coord: ChunkCoord) -> bool {
        self.resident.contains_key(&coord)
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
        let loaded = mgr
            .storage()
            .load_chunk(coord(1, 0, 0))
            .expect("load from disk");
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
        let file_size = std::fs::metadata(&path).expect("metadata").len();

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
        assert!(
            loaded.is_some(),
            "chunk should be loadable from disk after eviction"
        );
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
        assert!(
            dir.join("chunks").exists(),
            "chunks sub-directory should exist"
        );

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
                let mgr_storage = DiskChunkStorage::new(&base).expect("create storage per-thread");
                let mut mgr = StreamingManager::new(Box::new(mgr_storage), 5);

                for i in 0..10u32 {
                    let cx = (t * 10 + i as usize) as i32;
                    let result = mgr.request_chunk(coord(cx, 0, 0));
                    let _ = result.expect("concurrent request should not error");
                }
            }));
        }

        for h in handles {
            h.join().expect("thread should not panic");
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    // ========================================================================
    // LOD priority integration tests
    // ========================================================================

    /// STREAM-017 (test b): Eviction respects priority — close chunk is kept,
    /// far chunk is evicted first.
    #[test]
    fn eviction_respects_priority_close_kept_far_evicted() {
        let storage = Box::new(MemoryChunkStorage::new());
        let mut mgr = StreamingManager::with_lod_config(
            storage, 2, 2,  // vy_weight
            4,  // max_lod_level
            60, // recency_decay_ticks
            4,  // prefetch_budget
            1,  // mesh_ring
            3,  // prefetch_ring
        );

        // Set camera at origin
        mgr.set_camera_anchor(coord(0, 0, 0));

        // Insert a CLOSE chunk (ring=1) and a FAR chunk (ring=10)
        let close = sample_payload(1, 0, 0);
        let far = sample_payload(10, 0, 0);
        mgr.insert_chunk(close).expect("insert close");
        mgr.insert_chunk(far).expect("insert far");
        assert_eq!(mgr.resident_count(), 2);

        // Insert a third chunk — should evict the FAR chunk (lowest priority)
        let third = sample_payload(20, 0, 0);
        mgr.insert_chunk(third).expect("insert third");

        assert_eq!(mgr.resident_count(), 2);
        // The close chunk (1,0,0) should still be in the resident set
        assert!(
            mgr.is_resident(coord(1, 0, 0)),
            "close chunk should survive eviction"
        );
        // The far chunk (10,0,0) should have been evicted from the resident set
        assert!(
            !mgr.is_resident(coord(10, 0, 0)),
            "far chunk should be evicted first"
        );
        // The far chunk should have been saved to disk
        assert!(
            mgr.storage().chunk_exists(coord(10, 0, 0)),
            "evicted far chunk should be on disk"
        );
        // The newly inserted chunk should be in memory
        assert!(mgr.is_resident(coord(20, 0, 0)));
    }

    /// STREAM-018 (test f): Cache hit rate improvement with prefetch.
    /// Pre-populate disk, run prefetch, then verify chunks are resident.
    #[test]
    fn prefetch_improves_cache_hit_rate() {
        let dir = std::env::temp_dir().join("phenotype_gfx_test_stream018_prefetch");
        let _ = std::fs::remove_dir_all(&dir);
        let disk = DiskChunkStorage::new(&dir).expect("create storage");

        // Pre-populate disk with chunks at positions that will be predicted.
        // Camera at (0,0,0), velocity dx=3, future anchor at (3,0,0).
        // With mesh_ring=1, prefetch_ring=3, total_ring=4.
        // Place disk chunks at ring=1 from future anchor (3,0,0):
        //   (4,0,0), (2,0,0), (3,0,1), (3,0,-1)
        // All are ring > mesh_ring(1) from current anchor (0,0,0).
        let disk_chunks: Vec<ChunkCoord> = vec![
            coord(4, 0, 0),
            coord(2, 0, 0),
            coord(3, 0, 1),
            coord(3, 0, -1),
        ];
        for c in &disk_chunks {
            disk.save_chunk(&sample_payload(c.cx, c.cy, c.cz))
                .expect("save");
        }

        let mut mgr = StreamingManager::with_lod_config(
            Box::new(disk),
            20,
            2,
            4,
            60,
            8,
            1,
            3, // prefetch_budget=8 to ensure enough candidates
        );

        // Camera at origin, moving in +X direction
        mgr.set_camera_anchor(coord(0, 0, 0));
        mgr.set_camera_velocity(CameraVelocity {
            dx: 3,
            dy: 0,
            dz: 0,
        });

        // Before prefetch, nothing should be resident
        assert_eq!(mgr.resident_count(), 0);

        // Run prefetch — should load chunks from disk that are in the predicted path
        let prefetched = mgr.prefetch_next_frame().expect("prefetch");
        assert!(
            prefetched > 0,
            "prefetch should have loaded at least 1 chunk from disk, got {}",
            prefetched,
        );
        assert_eq!(mgr.stats().prefetch_hits as usize, prefetched);

        // After prefetch, those chunks should be resident
        // (request_chunk should hit in-memory, not disk)
        let hits_before = mgr.stats().disk_cache_hits;
        for c in &disk_chunks {
            let _ = mgr.request_chunk(*c).expect("request");
        }
        let hits_after = mgr.stats().disk_cache_hits;
        // At least some should have been in-memory hits (no additional disk reads)
        assert!(
            hits_after <= hits_before,
            "prefetched chunks should be in memory (no additional disk reads)"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// STREAM-019 (test g): Disk load gives priority boost — a chunk loaded
    /// from disk is marked as recently accessed and survives eviction over
    /// an older chunk at the same distance.
    #[test]
    fn disk_load_gives_priority_boost() {
        let dir = std::env::temp_dir().join("phenotype_gfx_test_stream019_boost");
        let _ = std::fs::remove_dir_all(&dir);
        let disk = DiskChunkStorage::new(&dir).expect("create storage");

        // Save a chunk on disk at (5, 0, 5) — ring=5 from origin
        disk.save_chunk(&sample_payload(5, 0, 5)).expect("save");

        let mut mgr = StreamingManager::with_lod_config(
            Box::new(DiskChunkStorage::new(&dir).expect("create storage2")),
            2, // max_resident = 2
            2,
            4,
            60,
            4,
            1,
            3,
        );
        mgr.set_camera_anchor(coord(0, 0, 0));

        // Insert chunk A at (5, 0, 0) — ring=5, same distance as disk chunk
        // at (5, 0, 5). Insert at tick 0 (older access).
        let chunk_a = sample_payload(5, 0, 0);
        mgr.insert_chunk(chunk_a).expect("insert A");

        // Advance 5 ticks
        for _ in 0..5 {
            mgr.advance_tick();
        }

        // Load disk chunk at (5, 0, 5) — gets recency boost at tick 5
        mgr.request_chunk(coord(5, 0, 5)).expect("load from disk");

        // Cache is full: A (tick 0) and disk-loaded (tick 5).
        // Insert a third — should evict chunk A (older access = lower recency)
        let chunk_c = sample_payload(10, 0, 0);
        mgr.insert_chunk(chunk_c).expect("insert C");

        // Chunk A should be evicted (older access = lower recency = lower priority)
        assert!(
            !mgr.is_resident(coord(5, 0, 0)),
            "chunk A (older access) should be evicted before disk-loaded chunk"
        );
        // Disk-loaded chunk should survive
        assert!(
            mgr.is_resident(coord(5, 0, 5)),
            "disk-loaded chunk with priority boost should survive"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// STREAM-020 (test h): No camera position → fallback to FIFO-like behavior
    /// where all chunks at the same distance get equal priority.
    #[test]
    fn no_camera_position_fallback_to_equal_priority() {
        let storage = Box::new(MemoryChunkStorage::new());
        let mut mgr = StreamingManager::with_lod_config(storage, 2, 2, 4, 60, 4, 1, 3);

        // Do NOT set camera anchor — camera_anchor is None
        assert!(mgr.camera_anchor().is_none());

        // Insert two chunks at equal distance from themselves (which is 0
        // since anchor defaults to the chunk itself when None).
        mgr.insert_chunk(sample_payload(1, 0, 0)).expect("insert 1");
        mgr.insert_chunk(sample_payload(2, 0, 0)).expect("insert 2");

        // Insert a third — should not panic, evicts one deterministically
        mgr.insert_chunk(sample_payload(3, 0, 0)).expect("insert 3");
        assert_eq!(mgr.resident_count(), 2);

        // No panic = fallback works correctly
    }

    /// STREAM-021: LOD level affects eviction — high LOD (coarse) chunk is
    /// evicted before low LOD (detailed) chunk at the same distance.
    #[test]
    fn high_lod_evicted_before_low_lod() {
        let storage = Box::new(MemoryChunkStorage::new());
        let mut mgr = StreamingManager::with_lod_config(storage, 2, 2, 4, 60, 4, 1, 3);
        mgr.set_camera_anchor(coord(0, 0, 0));

        // Two chunks at the SAME ring distance (ring=5), different LOD levels.
        // (5,0,0) has ring=5, (5,0,5) has ring=max(5,0,5)=5 with vy_weight=2.
        let lod0_payload = sample_payload(5, 0, 0);
        let lod4_payload = sample_payload(5, 0, 5);
        mgr.insert_chunk_with_lod(lod0_payload, 0)
            .expect("insert LOD 0");
        mgr.insert_chunk_with_lod(lod4_payload, 4)
            .expect("insert LOD 4");

        // Insert a third — should evict the LOD 4 chunk (coarse = low priority)
        mgr.insert_chunk(sample_payload(10, 0, 0))
            .expect("insert third");

        assert!(
            mgr.is_resident(coord(5, 0, 0)),
            "LOD 0 chunk should survive (higher detail = higher priority)"
        );
        assert!(
            !mgr.is_resident(coord(5, 0, 5)),
            "LOD 4 chunk should be evicted first (coarser = lower priority)"
        );
    }
}
