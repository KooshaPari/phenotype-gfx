// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 KooshaPari <kooshapari@gmail.com>

//! T40 spec: HDR LUT pipeline (Gen-5 RNP) hexagonal port.
//!
//! Supports 4 LUT formats: `.cube`, `.3dl`, `.csp`, and PNG-encoded Hald
//! images. Each format is an adapter; the pipeline can be swapped at
//! runtime via the config (e.g., "use `.cube` in editor, `.csp` in shipping").

use std::fmt;

use crate::postfx::error::PostFxResult;

/// Supported LUT (Look-Up Table) file formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LutFormat {
    /// Adobe Cube format.
    Cube,
    /// Discreet 3DL format.
    ThreeDl,
    /// CSP (Cspire) format.
    Csp,
    /// PNG-encoded Hald image.
    HaldPng,
}

impl fmt::Display for LutFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            LutFormat::Cube => "cube",
            LutFormat::ThreeDl => "3dl",
            LutFormat::Csp => "csp",
            LutFormat::HaldPng => "hald_png",
        })
    }
}

/// Parsed LUT data including format, size, HDR flag, color samples, and source
/// path. Engine-agnostic: the C# edge converts to a `Texture2D` on demand.
#[derive(Debug, Clone)]
pub struct LutData {
    /// File format of this LUT.
    pub format: LutFormat,
    /// Grid size of the LUT. Typically 17, 33, or 65 for `.cube` files;
    /// 8-256 for others.
    pub size: u32,
    /// Whether this LUT stores HDR values.
    pub is_hdr: bool,
    /// Flattened color array. Length is `size^3` for 3D LUTs or
    /// `size*size` for 1D LUTs.
    pub colors: Vec<[f32; 3]>,
    /// Source path or identifier of this LUT.
    pub source: String,
}

impl LutData {
    /// Creates an identity LUT that performs no color transformation.
    pub fn identity(format: LutFormat, size: u32) -> Self {
        let n = (size as usize).pow(3);
        let mut colors = Vec::with_capacity(n);
        let inv = 1.0_f32 / (size as f32 - 1.0).max(1.0);
        for b in 0..size {
            for g in 0..size {
                for r in 0..size {
                    colors.push([r as f32 * inv, g as f32 * inv, b as f32 * inv]);
                }
            }
        }
        Self {
            format,
            size,
            is_hdr: false,
            colors,
            source: "<identity>".into(),
        }
    }

    /// Returns `true` if the LUT is identity within the given tolerance.
    pub fn is_identity(&self, tolerance: f32) -> bool {
        let id = Self::identity(self.format, self.size);
        if id.colors.len() != self.colors.len() {
            return false;
        }
        for (a, b) in id.colors.iter().zip(self.colors.iter()) {
            for i in 0..3 {
                if (a[i] - b[i]).abs() > tolerance {
                    return false;
                }
            }
        }
        true
    }
}

/// Hexagonal port: parses and serializes a LUT in a specific format. Each
/// format (Cube, 3DL, CSP, Hald PNG) has its own adapter.
pub trait LutAdapter: Send + Sync {
    /// Format this adapter handles.
    fn format(&self) -> LutFormat;
    /// Parses a LUT file from the specified path.
    fn parse(&self, path: &str) -> PostFxResult<LutData>;
    /// Attempts to parse a LUT file; returns `None` on failure.
    fn try_parse(&self, path: &str) -> Option<LutData> {
        self.parse(path).ok()
    }
    /// Serializes LUT data to the specified path.
    fn serialize(&self, path: &str, data: &LutData) -> PostFxResult<()>;
}

/// Hexagonal port: the LUT pipeline (parse → validate → upload to GPU).
pub trait LutPipeline: Send + Sync {
    /// Currently loaded LUT data.
    fn current(&self) -> Option<&LutData>;
    /// Loads a LUT from the specified path.
    fn load(&mut self, path: &str) -> PostFxResult<()>;
    /// Replaces the current LUT with an identity transform (no color change).
    fn use_identity(&mut self);
    /// Validates the supplied LUT data.
    fn validate(&self, data: &LutData) -> Result<(), String>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lut_format_display() {
        assert_eq!(LutFormat::Cube.to_string(), "cube");
        assert_eq!(LutFormat::ThreeDl.to_string(), "3dl");
        assert_eq!(LutFormat::Csp.to_string(), "csp");
        assert_eq!(LutFormat::HaldPng.to_string(), "hald_png");
    }

    #[test]
    fn identity_lut_size() {
        let id = LutData::identity(LutFormat::Cube, 17);
        assert_eq!(id.size, 17);
        assert_eq!(id.colors.len(), 17 * 17 * 17);
        assert!(id.is_identity(0.001));
    }

    #[test]
    fn identity_lut_at_size_2() {
        let id = LutData::identity(LutFormat::Cube, 2);
        assert_eq!(id.colors.len(), 8);
        // Corner samples
        assert_eq!(id.colors[0], [0.0, 0.0, 0.0]);
        assert_eq!(id.colors[7], [1.0, 1.0, 1.0]);
    }

    #[test]
    fn is_identity_detects_modification() {
        let mut id = LutData::identity(LutFormat::Cube, 4);
        id.colors[0] = [0.5, 0.5, 0.5];
        assert!(!id.is_identity(0.001));
    }
}
