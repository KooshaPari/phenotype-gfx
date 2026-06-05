//! Chunk serialization — compact binary form for scene persistence.
//!
//! # Format (version 1)
//!
//! ```text
//! offset  size  description
//! ------  ----  -----------
//!  0       4    magic: b"PVOX"
//!  4       1    format version (currently 1)
//!  5       4    voxel element size in bytes (u32 LE) — guards mis-matched T
//!  9       4    RLE run count (u32 LE)
//! 13+      ?    RLE runs: each run is
//!                 [run_length: u16 LE][value bytes: element_size bytes]
//! ```
//!
//! **RLE contract:** run lengths sum to exactly [`CHUNK_VOXELS`] (4096).
//! A dense chunk of all-identical voxels serializes as a single 2+N-byte run.
//! A fully random chunk has 4096 runs of length 1.
//!
//! # Trait bounds
//!
//! The public API is generic over `T: bytemuck::Pod + Eq + Default + Clone`.
//! [`bytemuck::Pod`] provides safe `&[u8]` slicing of `T` values with no unsafe
//! code in this module. All built-in voxel types (`u8`, `u16`, [`MaterialId`],
//! etc.) satisfy this bound.
//!
//! [`MaterialId`]: crate::material::MaterialId

use std::io::{self, Read, Write};
use std::mem;

use bytemuck::Pod;

use crate::chunk::{Chunk, CHUNK_VOXELS};

/// Magic bytes at the head of every serialized chunk file.
const MAGIC: &[u8; 4] = b"PVOX";
/// Current format version written to every file.
const FORMAT_VERSION: u8 = 1;
/// Maximum run length that fits in a `u16`.
const MAX_RUN: u16 = u16::MAX;

// ────────────────────────────────────────────────────────────────────────────
// Public API
// ────────────────────────────────────────────────────────────────────────────

/// Serialize `chunk` into `w` using the compact PVOX RLE binary format.
///
/// The written bytes are fully self-describing: element size is embedded so a
/// reader can detect type mismatches before touching voxel data.
///
/// # Errors
///
/// Propagates any [`io::Error`] from `w`.
pub fn save_chunk<T, W>(chunk: &Chunk<T>, w: &mut W) -> io::Result<()>
where
    T: Pod + Eq + Default + Clone,
    W: Write,
{
    let elem = mem::size_of::<T>();

    // ── Header ──────────────────────────────────────────────────────────────
    w.write_all(MAGIC)?;
    w.write_all(&[FORMAT_VERSION])?;
    w.write_all(&u32::to_le_bytes(elem as u32))?;

    // ── RLE encode ──────────────────────────────────────────────────────────
    // Collect runs into a buffer so we can write the count first.
    let runs = rle_encode(&chunk.voxels);

    let run_count = u32::try_from(runs.len())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "too many RLE runs"))?;
    w.write_all(&u32::to_le_bytes(run_count))?;

    for (length, value) in &runs {
        w.write_all(&u16::to_le_bytes(*length))?;
        w.write_all(bytemuck::bytes_of(value))?;
    }

    Ok(())
}

/// Deserialize a [`Chunk<T>`] from `r` previously written by [`save_chunk`].
///
/// # Errors
///
/// Returns [`io::Error`] on:
/// - bad magic / unsupported version
/// - element-size mismatch (file was written with a different `T`)
/// - run lengths that do not sum to [`CHUNK_VOXELS`]
/// - any underlying I/O error from `r`
pub fn load_chunk<T, R>(r: &mut R) -> io::Result<Chunk<T>>
where
    T: Pod + Eq + Default + Clone,
    R: Read,
{
    let elem = mem::size_of::<T>();

    // ── Header ──────────────────────────────────────────────────────────────
    let mut magic = [0u8; 4];
    r.read_exact(&mut magic)?;
    if &magic != MAGIC {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "bad PVOX magic"));
    }

    let mut ver = [0u8; 1];
    r.read_exact(&mut ver)?;
    if ver[0] != FORMAT_VERSION {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("unsupported PVOX format version {}", ver[0]),
        ));
    }

    let file_elem = read_u32_le(r)? as usize;
    if file_elem != elem {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("element size mismatch: file={file_elem} bytes, requested={elem} bytes"),
        ));
    }

    // ── RLE decode ──────────────────────────────────────────────────────────
    let run_count = read_u32_le(r)? as usize;
    let mut voxels: Vec<T> = Vec::with_capacity(CHUNK_VOXELS);

    for _ in 0..run_count {
        let length = read_u16_le(r)? as usize;
        let mut raw = vec![0u8; elem];
        r.read_exact(&mut raw)?;
        // SAFETY: bytemuck::from_slice panics if alignment is wrong, but Pod
        // guarantees any alignment; we use try_from_bytes for a clean error.
        let value: &T = bytemuck::try_from_bytes(&raw)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("bytemuck: {e}")))?;
        for _ in 0..length {
            voxels.push(*value);
        }
    }

    if voxels.len() != CHUNK_VOXELS {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "RLE sum mismatch: got {} voxels, expected {CHUNK_VOXELS}",
                voxels.len()
            ),
        ));
    }

    Ok(Chunk { voxels })
}

// ────────────────────────────────────────────────────────────────────────────
// Internal helpers
// ────────────────────────────────────────────────────────────────────────────

/// Encode `voxels` as `(run_length, value)` pairs.
///
/// Runs are capped at [`MAX_RUN`] so lengths always fit in `u16`.
fn rle_encode<T: Eq + Copy>(voxels: &[T]) -> Vec<(u16, T)> {
    if voxels.is_empty() {
        return Vec::new();
    }

    let mut runs: Vec<(u16, T)> = Vec::new();
    let mut current = voxels[0];
    let mut count: u16 = 1;

    for &v in &voxels[1..] {
        if v == current && count < MAX_RUN {
            count += 1;
        } else {
            runs.push((count, current));
            current = v;
            count = 1;
        }
    }
    runs.push((count, current));
    runs
}

#[inline]
fn read_u32_le<R: Read>(r: &mut R) -> io::Result<u32> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}

#[inline]
fn read_u16_le<R: Read>(r: &mut R) -> io::Result<u16> {
    let mut buf = [0u8; 2];
    r.read_exact(&mut buf)?;
    Ok(u16::from_le_bytes(buf))
}

// ────────────────────────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// FR-PHENO-VOXEL-SERIAL-000 — empty (all-zero) chunk roundtrips losslessly.
    ///
    /// An "empty" chunk in the voxel kernel is a chunk where every voxel is
    /// `T::default()`, i.e. `MaterialId(0)` / air. RLE should encode it as a
    /// single run of length 4096.
    #[test]
    fn empty_chunk_roundtrip() {
        let original: Chunk<u8> = Chunk::default();
        assert_eq!(original.voxels.len(), CHUNK_VOXELS);
        assert!(original.voxels.iter().all(|&v| v == 0));

        let mut buf = Vec::<u8>::new();
        save_chunk(&original, &mut buf).expect("save_chunk must not fail");

        // With RLE a fully-uniform chunk should serialize very compactly:
        // 4 (magic) + 1 (ver) + 4 (elem_size) + 4 (run_count) + 2+1 (one run)
        // = 16 bytes for u8.
        assert_eq!(buf.len(), 4 + 1 + 4 + 4 + 3, "expected single-run encoding");

        let recovered: Chunk<u8> =
            load_chunk(&mut buf.as_slice()).expect("load_chunk must not fail");
        assert_eq!(
            recovered.voxels, original.voxels,
            "roundtrip must be lossless"
        );
    }

    /// FR-PHENO-VOXEL-SERIAL-001 — dense (all-distinct) chunk roundtrips losslessly.
    ///
    /// Fills the chunk with a cycling byte pattern so every adjacent pair of
    /// voxels differs, exercising the maximum-runs path of the RLE encoder.
    #[test]
    fn dense_chunk_roundtrip() {
        let mut original: Chunk<u8> = Chunk::default();
        for (i, v) in original.voxels.iter_mut().enumerate() {
            // Alternating 0/1 ensures no two adjacent voxels are equal →
            // forces 4096 individual runs of length 1.
            *v = (i % 2) as u8;
        }

        let mut buf = Vec::<u8>::new();
        save_chunk(&original, &mut buf).expect("save_chunk must not fail");

        let recovered: Chunk<u8> =
            load_chunk(&mut buf.as_slice()).expect("load_chunk must not fail");
        assert_eq!(
            recovered.voxels, original.voxels,
            "roundtrip must be lossless"
        );
    }

    /// FR-PHENO-VOXEL-SERIAL-002 — bad magic is rejected with InvalidData.
    #[test]
    fn bad_magic_rejected() {
        let buf = b"NOPE\x01\x01\x00\x00\x00\x00\x00\x00\x00".to_vec();
        let result: io::Result<Chunk<u8>> = load_chunk(&mut buf.as_slice());
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidData);
    }

    /// FR-PHENO-VOXEL-SERIAL-003 — element-size mismatch is detected.
    #[test]
    fn element_size_mismatch_detected() {
        // Write a u8 chunk, then try to read it back as u16.
        let original: Chunk<u8> = Chunk::default();
        let mut buf = Vec::<u8>::new();
        save_chunk(&original, &mut buf).expect("save_chunk");

        let result: io::Result<Chunk<u16>> = load_chunk(&mut buf.as_slice());
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidData);
    }
}
