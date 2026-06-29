//! Typed errors for the water module.
//!
//! `Io` and `Json` variants are not `PartialEq` natively (`std::io::Error` and
//! `serde_json::Error` aren't comparable), so we implement `PartialEq` manually
//! via `to_string()` comparison.

use serde_json;
use std::fmt;
use std::io;

/// All recoverable errors from the water module.
#[derive(Debug)]
pub enum WaterError {
    /// A spatial / out-of-bounds violation. Mirrors `ArgumentOutOfRangeException`.
    OutOfBounds {
        /// Human-readable description.
        msg: String,
    },
    /// Length-mismatch on an input array.
    InvalidDataLength {
        /// Actual length of the input.
        got: usize,
        /// Length the operation expected.
        expected: usize,
    },
    /// Wrapped `std::io::Error` from a serialization round-trip.
    Io(io::Error),
    /// Wrapped `serde_json::Error` from JSON serialization.
    Json(serde_json::Error),
    /// A material was passed as `null` to a call that required a real instance.
    NullMaterial,
    /// A shader was passed as `null` to a call that required a real instance.
    NullShader,
    /// A wave bank was passed as `null` to a call that required a real instance.
    NullWaveBank,
    /// A distance / threshold comparison failed.
    InvalidDistance {
        /// The observed distance.
        value: f32,
    },
    /// Threshold configuration is invalid (e.g. low ≥ high).
    InvalidThresholds {
        /// Human-readable description of the constraint that was violated.
        msg: String,
    },
    /// A required field was missing in a deserialized snapshot.
    MissingField {
        /// Field name.
        name: &'static str,
    },
}

impl fmt::Display for WaterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutOfBounds { msg } => write!(f, "Out of range: {}", msg),
            Self::InvalidDataLength { got, expected } => {
                write!(f, "Invalid data length: got {}, expected {}", got, expected)
            }
            Self::Io(e) => write!(f, "I/O error: {}", e),
            Self::Json(e) => write!(f, "JSON error: {}", e),
            Self::NullMaterial => write!(f, "WaterMaterial must not be null"),
            Self::NullShader => write!(f, "WaterShader must not be null"),
            Self::NullWaveBank => write!(f, "GerstnerWaveBank must not be null"),
            Self::InvalidDistance { value } => {
                write!(f, "Distance mismatch: {} out of range", value)
            }
            Self::InvalidThresholds { msg } => write!(f, "Invalid thresholds: {}", msg),
            Self::MissingField { name } => write!(f, "Missing required field: {}", name),
        }
    }
}

impl std::error::Error for WaterError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Json(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for WaterError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}
impl From<serde_json::Error> for WaterError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}
impl From<crate::terrain::error::TerrainError> for WaterError {
    fn from(e: crate::terrain::error::TerrainError) -> Self {
        use crate::terrain::error::TerrainError as T;
        match e {
            T::OutOfBounds { msg } => Self::OutOfBounds { msg },
            T::InvalidDataLength { got, expected } => Self::InvalidDataLength { got, expected },
            T::InvalidDistance { value } => Self::InvalidDistance { value },
            T::InvalidThresholds { msg } => Self::InvalidThresholds { msg },
            T::MaterialNotFound(name) => Self::OutOfBounds {
                msg: format!("terrain material not found: {name}"),
            },
            T::NullMaterial => Self::NullMaterial,
            T::Io(e) => Self::Io(e),
            T::Json(e) => Self::Json(e),
            T::InvalidResolution { .. } => Self::OutOfBounds {
                msg: "Invalid resolution (from terrain LOD)".to_string(),
            },
        }
    }
}

/// Result alias used throughout the water module.
pub type WaterResult<T> = Result<T, WaterError>;

impl PartialEq for WaterError {
    fn eq(&self, other: &Self) -> bool {
        // Compare by display string. Cheap, deterministic, and matches how
        // the C# tests assert on `ArgumentException.Message`.
        self.to_string() == other.to_string()
    }
}
