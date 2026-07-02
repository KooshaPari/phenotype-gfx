// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 KooshaPari <kooshapari@gmail.com>

//! Typed errors for the post-processing stack.
//!
//! Mirrors the C# `WaterError` / `TerrainError` pattern: a `thiserror` enum
//! with manual `PartialEq` (via `to_string()` comparison for `Io` / `Json`
//! variants) so error values can be used as `assert_eq!` arguments in tests.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum PostFxError {
    #[error("shader variant unavailable: {shader_name} ({keyword})")]
    ShaderVariantUnavailable {
        shader_name: String,
        keyword: String,
    },

    #[error("invalid pass descriptor: {0}")]
    InvalidPassDescriptor(String),

    #[error("invalid LUT: {0}")]
    InvalidLut(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("unknown post-fx effect: {0}")]
    UnknownEffect(String),
}

pub type PostFxResult<T> = std::result::Result<T, PostFxError>;

impl Clone for PostFxError {
    fn clone(&self) -> Self {
        match self {
            PostFxError::ShaderVariantUnavailable {
                shader_name,
                keyword,
            } => PostFxError::ShaderVariantUnavailable {
                shader_name: shader_name.clone(),
                keyword: keyword.clone(),
            },
            PostFxError::InvalidPassDescriptor(s) => PostFxError::InvalidPassDescriptor(s.clone()),
            PostFxError::InvalidLut(s) => PostFxError::InvalidLut(s.clone()),
            PostFxError::Io(e) => PostFxError::Io(std::io::Error::new(e.kind(), e.to_string())),
            PostFxError::Json(e) => {
                // serde_json::Error has no public Clone; use a string + line/col
                // reconstruction. Round-trip via the line/col constructor.
                PostFxError::Json(serde_json::Error::io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                )))
            }
            PostFxError::UnknownEffect(s) => PostFxError::UnknownEffect(s.clone()),
        }
    }
}

impl PartialEq for PostFxError {
    fn eq(&self, other: &Self) -> bool {
        self.to_string() == other.to_string()
    }
}

impl Eq for PostFxError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_error_partial_eq_via_display() {
        let a = PostFxError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "x"));
        let b = PostFxError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "x"));
        assert_eq!(a, b);
    }

    #[test]
    fn shader_variant_error_clone() {
        let e = PostFxError::ShaderVariantUnavailable {
            shader_name: "Bloom".into(),
            keyword: "BLOOM_LOW".into(),
        };
        assert_eq!(e, e.clone());
    }
}
