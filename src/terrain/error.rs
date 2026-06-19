//! Typed errors for the terrain module.
//!
//! `PartialEq` is implemented manually with a `to_string()` comparison for
//! `Io`/`Json` variants because `std::io::Error` and `serde_json::Error` are
//! not `PartialEq`. This is the conventional approach when you want an error
//! enum to be `PartialEq` for test assertions without comparing internals.

use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TerrainError {
    /// Coord was out of `[0, width)` or `[0, height)`.
    #[error("out of bounds: {msg}")]
    OutOfBounds {
        /// Human-readable explanation.
        msg: String,
    },

    /// `data.len()` did not equal `width * height`.
    #[error("invalid data length: got {got}, expected {expected}")]
    InvalidDataLength {
        /// Length actually supplied.
        got: usize,
        /// Length that was required (`width * height`).
        expected: usize,
    },

    /// Underlying IO failure.
    #[error("io: {0}")]
    Io(#[from] io::Error),

    /// JSON parsing/serialization failure.
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),

    /// `resolution` was `<= 0`.
    #[error("invalid resolution: {value} (must be > 0)")]
    InvalidResolution {
        /// The rejected resolution.
        value: i32,
    },

    /// `distance` was negative.
    #[error("invalid distance: {value} (must be >= 0)")]
    InvalidDistance {
        /// The rejected distance.
        value: f32,
    },

    /// LOD thresholds were not monotonically increasing.
    #[error("invalid LOD thresholds: {msg}")]
    InvalidThresholds {
        /// Human-readable explanation.
        msg: String,
    },

    /// Material not in registry.
    #[error("material not found: {0}")]
    MaterialNotFound(String),

    /// A `null` material was passed where a real one was required.
    #[error("null material")]
    NullMaterial,
}

impl PartialEq for TerrainError {
    fn eq(&self, other: &Self) -> bool {
        // Compare by string representation so we don't reach into
        // non-PartialEq internals of io::Error / serde_json::Error.
        self.to_string() == other.to_string()
    }
}

pub type TerrainResult<T> = Result<T, TerrainError>;
