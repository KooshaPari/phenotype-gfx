// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 KooshaPari <kooshapari@gmail.com>

//! `IShaderAvailabilityProvider` — port for shader-variant detection.
//!
//! Adapters query this to validate that all required shader variants are loaded
//! before the pass runs (otherwise the pass would no-op or assert at runtime).

use std::collections::HashMap;
use std::sync::Arc;

/// Hexagonal port: asks the platform "is this shader available right now?".
///
/// Adapters include [`DefaultPostFxShaderAvailability`] (built-in),
/// `AddressablesShaderProvider` (future), and `AssetBundleShaderProvider` (future).
pub trait PostFxShaderAvailability: Send + Sync {
    /// Determines whether a shader with the specified name and keyword is
    /// available.
    fn is_available(&self, shader_name: &str, keyword: &str) -> bool;
}

/// Default implementation that always returns `true`.
///
/// Suitable for built-in render pipelines where all shader variants are
/// guaranteed.
#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultPostFxShaderAvailability;

impl PostFxShaderAvailability for DefaultPostFxShaderAvailability {
    fn is_available(&self, _shader_name: &str, _keyword: &str) -> bool {
        true
    }
}

/// Map-based availability provider: per (shader, keyword) → available.
///
/// Useful in tests to selectively disable a single variant.
#[derive(Debug, Default, Clone)]
pub struct MapPostFxShaderAvailability {
    map: HashMap<(String, String), bool>,
}

impl MapPostFxShaderAvailability {
    /// New empty map-based provider.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the availability for a single (shader, keyword) pair.
    pub fn set(&mut self, shader: impl Into<String>, keyword: impl Into<String>, available: bool) {
        self.map.insert((shader.into(), keyword.into()), available);
    }

    /// Convenience: all available.
    pub fn all_available() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Convenience: all unavailable.
    pub fn none_available() -> Arc<Self> {
        let mut m = Self::new();
        for v in [
            ("ScreenSpaceAO", "SSAOPASS"),
            ("ScreenSpaceGI", "SSGIPASS"),
            ("BrpBloom", "BLOOM_HIGH"),
            ("BrpACES", "ACES"),
            ("Vignette", "VIGNETTE"),
            ("ChromaticAberration", "CHROMATIC"),
            ("ColorGradingLUT", "LUT"),
        ] {
            m.set(v.0, v.1, false);
        }
        Arc::new(m)
    }
}

impl PostFxShaderAvailability for MapPostFxShaderAvailability {
    fn is_available(&self, shader_name: &str, keyword: &str) -> bool {
        self.map
            .get(&(shader_name.to_string(), keyword.to_string()))
            .copied()
            .unwrap_or(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_provider_always_true() {
        let p = DefaultPostFxShaderAvailability;
        assert!(p.is_available("anything", "anywhere"));
    }

    #[test]
    fn map_provider_explicit_value() {
        let mut p = MapPostFxShaderAvailability::new();
        p.set("BrpBloom", "BLOOM_LOW", false);
        assert!(!p.is_available("BrpBloom", "BLOOM_LOW"));
        // Default: true for unset entries.
        assert!(p.is_available("BrpBloom", "BLOOM_HIGH"));
    }
}
