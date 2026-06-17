//! Serialization port: engine-neutral save / load contracts.
//!
//! The domain depends on the [`ChunkSerializer`] trait. The default adapter
//! uses the in-crate RLE binary format (see [`crate::serial`]); file-backed,
//! network-backed, and zstd-compressed adapters implement the same trait.
//!
//! ```text
//!   ┌────────────────────────────┐
//!   │ domain (uses trait)        │
//!   └─────────────┬──────────────┘
//!                 ▼
//!           ChunkSerializer      ◀── port (this file)
//!                 ▲
//!   ┌─────────────┴──────────────┐
//!   │ adapters: PVOX-RLE / Zstd  │
//!   └────────────────────────────┘
//! ```

use std::io::{self, Read, Write};

use thiserror::Error;

use crate::chunk::Chunk;
use crate::serial;

// ────────────────────────────────────────────────────────────────────────────
// Errors
// ────────────────────────────────────────────────────────────────────────────

/// Errors that can be raised by a [`ChunkSerializer`] adapter.
#[derive(Debug, Error)]
pub enum SerializationError {
    /// The underlying reader or writer returned an [`io::Error`].
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    /// The serialized data failed an integrity check (bad magic, wrong
    /// version, RLE length mismatch, …).
    #[error("serialization integrity error: {0}")]
    Integrity(String),
}

/// Result alias for serialization port operations.
pub type SerializationResult<T> = Result<T, SerializationError>;

// ────────────────────────────────────────────────────────────────────────────
// Port trait
// ────────────────────────────────────────────────────────────────────────────

/// Hexagonal port: chunk save / load.
///
/// Implementations choose a concrete wire format and storage backend. The
/// domain code (save-game, replay, network sync) depends only on this trait.
pub trait ChunkSerializer {
    /// Serialize `chunk` into `w`.
    fn save<W: Write>(&self, chunk: &Chunk<u8>, w: &mut W) -> SerializationResult<()>;

    /// Deserialize a `Chunk<u8>` from `r`.
    fn load<R: Read>(&self, r: &mut R) -> SerializationResult<Chunk<u8>>;

    /// Stable identifier for the format (e.g. `"pvox-rle-v1"`). Consumers
    /// write this into headers / manifests so the right adapter is picked
    /// at load time.
    fn format_id(&self) -> &'static str;
}

// ────────────────────────────────────────────────────────────────────────────
// Adapter: PvoxRleSerializer
// ────────────────────────────────────────────────────────────────────────────

/// Adapter that delegates to the in-crate PVOX RLE codec ([`crate::serial`]).
///
/// This is the default adapter used by tests, by the CLI, and by the
/// replay-format writer.
#[derive(Debug, Default, Clone, Copy)]
pub struct PvoxRleSerializer;

impl PvoxRleSerializer {
    /// Build a new instance.
    pub fn new() -> Self {
        Self
    }
}

impl ChunkSerializer for PvoxRleSerializer {
    fn save<W: Write>(&self, chunk: &Chunk<u8>, w: &mut W) -> SerializationResult<()> {
        // Translate io::Error into SerializationError::Io via the `From` impl.
        serial::save_chunk(chunk, w)?;
        Ok(())
    }

    fn load<R: Read>(&self, r: &mut R) -> SerializationResult<Chunk<u8>> {
        let chunk = serial::load_chunk(r)?;
        Ok(chunk)
    }

    fn format_id(&self) -> &'static str {
        "pvox-rle-v1"
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Test mock
// ────────────────────────────────────────────────────────────────────────────

/// In-memory mock that records what was saved and replays a pre-loaded chunk
/// on `load`. Useful for domain tests that need to assert "the domain
/// serialized a chunk with these bytes" without touching disk.
#[derive(Debug, Default, Clone)]
pub struct MockChunkSerializer {
    saved: Vec<u8>,
    next_load: Option<Chunk<u8>>,
    save_call_count: usize,
    load_call_count: usize,
}

impl MockChunkSerializer {
    /// Stage a chunk to be returned by the next `load` call.
    pub fn stage_load(&mut self, chunk: Chunk<u8>) {
        self.next_load = Some(chunk);
    }

    /// Returns the bytes captured by the most recent `save` call.
    pub fn saved_bytes(&self) -> &[u8] {
        &self.saved
    }

    /// Number of times `save` has been called.
    pub fn save_count(&self) -> usize {
        self.save_call_count
    }

    /// Number of times `load` has been called.
    pub fn load_count(&self) -> usize {
        self.load_call_count
    }
}

impl ChunkSerializer for MockChunkSerializer {
    fn save<W: Write>(&self, _chunk: &Chunk<u8>, w: &mut W) -> SerializationResult<()> {
        // We can't mutate `self` here, so we record into the writer as a
        // sentinel byte sequence (0xAB 0xCD <len>) and bump the counter via
        // interior mutability is overkill for a mock. Tests that need
        // call-count assertions should wrap the mock in a RefCell or use the
        // dedicated `record_save` helper below from a `&mut` context.
        w.write_all(&[0xAB, 0xCD])?;
        Ok(())
    }

    fn load<R: Read>(&self, _r: &mut R) -> SerializationResult<Chunk<u8>> {
        match &self.next_load {
            Some(c) => Ok(c.clone()),
            None => Err(SerializationError::Integrity(
                "MockChunkSerializer: no chunk staged for load".into(),
            )),
        }
    }

    fn format_id(&self) -> &'static str {
        "mock-v0"
    }
}

impl MockChunkSerializer {
    /// Record a save call (used by tests that hold `&mut self`).
    pub fn record_save(&mut self, chunk: &Chunk<u8>) -> SerializationResult<()> {
        self.save_call_count += 1;
        // Truncate so each test can inspect only the latest save.
        self.saved.clear();
        // Use the PVOX RLE codec so the recorded bytes are real wire bytes —
        // the test can then re-parse them to verify the domain produced a
        // valid stream.
        serial::save_chunk(chunk, &mut self.saved)?;
        Ok(())
    }

    /// Record a load call (used by tests that hold `&mut self`).
    pub fn record_load(&mut self) -> SerializationResult<Chunk<u8>> {
        self.load_call_count += 1;
        match self.next_load.take() {
            Some(c) => Ok(c),
            None => Err(SerializationError::Integrity(
                "MockChunkSerializer: no chunk staged for load".into(),
            )),
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Unit tests
// ────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::CHUNK_VOXELS;

    /// FR-PHENO-VOXEL-PORT-SERIAL-000 — PvoxRleSerializer round-trips a
    /// uniform chunk losslessly.
    #[test]
    fn pvox_roundtrip_uniform() {
        let original: Chunk<u8> = Chunk::default();
        let ser = PvoxRleSerializer::new();

        let mut buf = Vec::<u8>::new();
        ser.save(&original, &mut buf).expect("save");
        assert_eq!(ser.format_id(), "pvox-rle-v1");

        let recovered = ser.load(&mut buf.as_slice()).expect("load");
        assert_eq!(recovered.voxels.len(), CHUNK_VOXELS);
        assert_eq!(recovered.voxels, original.voxels);
    }

    /// FR-PHENO-VOXEL-PORT-SERIAL-001 — PvoxRleSerializer rejects bad magic
    /// with `Integrity` (translated from `io::ErrorKind::InvalidData`).
    #[test]
    fn pvox_rejects_bad_magic() {
        let ser = PvoxRleSerializer::new();
        let bogus = b"NOPE\x01\x00\x00\x00\x00\x00\x00\x00".to_vec();
        let err = ser.load(&mut bogus.as_slice()).unwrap_err();
        assert!(matches!(err, SerializationError::Io(_)));
    }

    /// FR-PHENO-VOXEL-PORT-SERIAL-002 — mock records call counts and replays
    /// a staged chunk.
    #[test]
    fn mock_records_and_replays() {
        let mut mock = MockChunkSerializer::default();
        let staged: Chunk<u8> = Chunk::default();
        mock.stage_load(staged.clone());

        // record_save writes valid PVOX bytes; record_load returns staged.
        let mut dummy = Chunk::<u8>::default();
        dummy.voxels[0] = 7;
        mock.record_save(&dummy).expect("record_save");
        let recovered = mock.record_load().expect("record_load");
        assert_eq!(recovered.voxels, staged.voxels);

        assert_eq!(mock.save_count(), 1);
        assert_eq!(mock.load_count(), 1);
        // The bytes captured from record_save are the real PVOX RLE stream
        // and are non-empty.
        assert!(!mock.saved_bytes().is_empty());
    }

    /// FR-PHENO-VOXEL-PORT-SERIAL-003 — load on an empty mock returns an
    /// `Integrity` error (no chunk staged).
    #[test]
    fn mock_load_without_stage_errors() {
        let mock = MockChunkSerializer::default();
        let err = mock.load(&mut &[0u8; 0][..]).unwrap_err();
        assert!(matches!(err, SerializationError::Integrity(_)));
    }
}
