// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 KooshaPari <kooshapari@gmail.com>

//! `PostStack` — central driver for the post-processing stack.
//!
//! Engine-agnostic port from the C# `PostStack.cs` (MonoBehaviour). The Rust
//! side holds the configuration, the validated availability flags, and the
//! pass registry. The C# edge binds the registry to a `MonoBehaviour` at
//! runtime and dispatches `on_render` for each pass in render order.
//!
//! The `describe_passes()` method returns a `Vec<PassDescriptor>` so that
//! the editor / driver can build an inspector / dispatcher without referring
//! to the concrete `BloomPass` / `SsaoPass` types.

use serde::{Deserialize, Serialize};

use crate::postfx::bloom_pass::BloomConfig;
use crate::postfx::ports::post_fx_pass::{
    PassDescriptor, PassEffect, PassQuality, PostFxContext, PostFxPass,
};
use crate::postfx::ports::shader_availability::PostFxShaderAvailability;
use crate::postfx::post_fx_pass_registry::PostFxPassRegistry;
use crate::postfx::ssao_pass::SsaoConfig;

/// All post-fx configuration. Engine-agnostic: carries only logical state,
/// not Unity references.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PostStackConfig {
    /// SSAO toggle.
    pub enable_ssao: bool,
    /// SSGI toggle.
    pub enable_ssgi: bool,
    /// Bloom toggle.
    pub enable_bloom: bool,
    /// ACES toggle.
    pub enable_aces: bool,
    /// Vignette toggle.
    pub enable_vignette: bool,
    /// Chromatic aberration toggle.
    pub enable_chromatic_aberration: bool,
    /// LUT toggle.
    pub enable_lut: bool,

    /// Overall quality preset.
    pub quality: PassQuality,

    /// SSAO sample count.
    pub ssao_samples: u32,
    /// SSAO world-space radius.
    pub ssao_radius: f32,
    /// SSAO depth bias.
    pub ssao_bias: f32,
    /// SSAO intensity.
    pub ssao_intensity: f32,

    /// SSGI sample count.
    pub ssgi_samples: u32,
    /// SSGI world-space radius.
    pub ssgi_radius: f32,
    /// SSGI intensity.
    pub ssgi_intensity: f32,

    /// Exposure for ACES.
    pub exposure: f32,
    /// Vignette intensity.
    pub vignette_intensity: f32,
    /// Vignette smoothness.
    pub vignette_smoothness: f32,
    /// Vignette roundness.
    pub vignette_roundness: f32,
    /// Vignette center (normalized).
    pub vignette_center: [f32; 2],
    /// Chromatic aberration intensity.
    pub chromatic_aberration_intensity: f32,
}

impl Default for PostStackConfig {
    fn default() -> Self {
        Self {
            enable_ssao: true,
            enable_ssgi: false,
            enable_bloom: false,
            enable_aces: true,
            enable_vignette: false,
            enable_chromatic_aberration: false,
            enable_lut: true,
            quality: PassQuality::High,
            ssao_samples: 12,
            ssao_radius: 2.0,
            ssao_bias: 0.0012,
            ssao_intensity: 1.0,
            ssgi_samples: 12,
            ssgi_radius: 1.8,
            ssgi_intensity: 0.45,
            exposure: 1.0,
            vignette_intensity: 0.45,
            vignette_smoothness: 0.6,
            vignette_roundness: 1.0,
            vignette_center: [0.5, 0.5],
            chromatic_aberration_intensity: 0.15,
        }
    }
}

impl PostStackConfig {
    /// Build the bloom sub-config from the post-stack config.
    pub fn bloom_config(&self) -> BloomConfig {
        BloomConfig {
            is_enabled: self.enable_bloom,
            ..BloomConfig::default()
        }
    }

    /// Build the SSAO sub-config from the post-stack config.
    pub fn ssao_config(&self) -> SsaoConfig {
        SsaoConfig {
            is_enabled: self.enable_ssao,
            radius: self.ssao_radius,
            intensity: self.ssao_intensity,
            bias: self.ssao_bias,
            kernel_size: self.ssao_samples,
        }
    }
}

/// Central driver for the post-processing stack.
///
/// Engine-agnostic; the C# edge wraps this in a `MonoBehaviour` and calls
/// `on_render` for each pass during `OnRenderImage`.
pub struct PostStack {
    config: PostStackConfig,
    registry: PostFxPassRegistry,
    /// Validated availability flags — set by `validate_shader_variants`.
    ssao_supported: bool,
    ssgi_supported: bool,
    bloom_supported: bool,
    aces_supported: bool,
    vignette_supported: bool,
    chromatic_aberration_supported: bool,
    lut_supported: bool,
}

/// Default-constructed post-stack used as the canonical config baseline.
pub const DEFAULT_POSTFX_STACK: PostStackConfig = PostStackConfig {
    enable_ssao: true,
    enable_ssgi: false,
    enable_bloom: false,
    enable_aces: true,
    enable_vignette: false,
    enable_chromatic_aberration: false,
    enable_lut: true,
    quality: PassQuality::High,
    ssao_samples: 12,
    ssao_radius: 2.0,
    ssao_bias: 0.0012,
    ssao_intensity: 1.0,
    ssgi_samples: 12,
    ssgi_radius: 1.8,
    ssgi_intensity: 0.45,
    exposure: 1.0,
    vignette_intensity: 0.45,
    vignette_smoothness: 0.6,
    vignette_roundness: 1.0,
    vignette_center: [0.5, 0.5],
    chromatic_aberration_intensity: 0.15,
};

impl PostStack {
    /// New post-stack with the given config and an empty registry.
    pub fn new(config: PostStackConfig) -> Self {
        Self {
            config,
            registry: PostFxPassRegistry::new(),
            ssao_supported: false,
            ssgi_supported: false,
            bloom_supported: false,
            aces_supported: false,
            vignette_supported: false,
            chromatic_aberration_supported: false,
            lut_supported: false,
        }
    }

    /// Borrow the current config.
    pub fn config(&self) -> &PostStackConfig {
        &self.config
    }

    /// Mutably borrow the current config.
    pub fn config_mut(&mut self) -> &mut PostStackConfig {
        &mut self.config
    }

    /// Borrow the registry.
    pub fn registry(&self) -> &PostFxPassRegistry {
        &self.registry
    }

    /// Mutably borrow the registry.
    pub fn registry_mut(&mut self) -> &mut PostFxPassRegistry {
        &mut self.registry
    }

    /// Returns the static descriptors for the built-in passes, in the
    /// canonical render order. The driver / editor uses this to build an
    /// inspector / dispatcher without referring to the concrete `BloomPass`
    /// / `SsaoPass` types.
    pub fn describe_passes() -> Vec<PassDescriptor> {
        vec![
            SsaoConfig::descriptor(),
            BloomConfig::descriptor(),
        ]
    }

    /// Audits each effect against the availability provider and updates the
    /// per-effect support flags.
    pub fn validate_shader_variants(&mut self, provider: &dyn PostFxShaderAvailability) {
        self.ssao_supported =
            provider.is_available("ScreenSpaceAO", "SSAOPASS");
        self.ssgi_supported =
            provider.is_available("ScreenSpaceGI", "SSGIPASS");
        self.bloom_supported =
            provider.is_available("BrpBloom", "BLOOM_HIGH");
        self.aces_supported =
            provider.is_available("BrpACES", "ACES");
        self.vignette_supported =
            provider.is_available("Vignette", "VIGNETTE");
        self.chromatic_aberration_supported =
            provider.is_available("ChromaticAberration", "CHROMATIC");
        self.lut_supported =
            provider.is_available("ColorGradingLUT", "LUT");
    }

    /// Returns `true` if the SSAO shader variant is available.
    pub fn ssao_supported(&self) -> bool { self.ssao_supported }
    /// Returns `true` if the SSGI shader variant is available.
    pub fn ssgi_supported(&self) -> bool { self.ssgi_supported }
    /// Returns `true` if the Bloom shader variant is available.
    pub fn bloom_supported(&self) -> bool { self.bloom_supported }
    /// Returns `true` if the ACES shader variant is available.
    pub fn aces_supported(&self) -> bool { self.aces_supported }
    /// Returns `true` if the Vignette shader variant is available.
    pub fn vignette_supported(&self) -> bool { self.vignette_supported }
    /// Returns `true` if the ChromaticAberration shader variant is available.
    pub fn chromatic_aberration_supported(&self) -> bool { self.chromatic_aberration_supported }
    /// Returns `true` if the LUT shader variant is available.
    pub fn lut_supported(&self) -> bool { self.lut_supported }
}

impl Default for PostStack {
    fn default() -> Self {
        Self::new(PostStackConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::postfx::ports::shader_availability::{DefaultPostFxShaderAvailability, MapPostFxShaderAvailability};

    #[test]
    fn default_config_baseline() {
        let s = PostStack::default();
        let cfg = s.config();
        assert!(cfg.enable_ssao);
        assert!(!cfg.enable_ssgi);
        assert!(!cfg.enable_bloom);
        assert!(cfg.enable_aces);
        assert!(!cfg.enable_vignette);
        assert!(!cfg.enable_chromatic_aberration);
        assert!(cfg.enable_lut);
        assert_eq!(cfg.quality, PassQuality::High);
    }

    #[test]
    fn describe_passes_includes_built_ins() {
        let descs = PostStack::describe_passes();
        assert!(descs.iter().any(|d| d.effect == PassEffect::Bloom));
        assert!(descs.iter().any(|d| d.effect == PassEffect::Ssao));
    }

    #[test]
    fn validate_all_supported_with_default() {
        let mut s = PostStack::default();
        let p = DefaultPostFxShaderAvailability;
        s.validate_shader_variants(&p);
        assert!(s.ssao_supported());
        assert!(s.bloom_supported());
        assert!(s.aces_supported());
        assert!(s.vignette_supported());
        assert!(s.chromatic_aberration_supported());
        assert!(s.lut_supported());
        assert!(s.ssgi_supported());
    }

    #[test]
    fn validate_with_none_available() {
        let mut s = PostStack::default();
        let p = MapPostFxShaderAvailability::none_available();
        s.validate_shader_variants(&*p);
        assert!(!s.ssao_supported());
        assert!(!s.ssgi_supported());
        assert!(!s.bloom_supported());
        assert!(!s.aces_supported());
        assert!(!s.vignette_supported());
        assert!(!s.chromatic_aberration_supported());
        assert!(!s.lut_supported());
    }

    #[test]
    fn bloom_and_ssao_config_derived() {
        let cfg = PostStackConfig::default();
        let b = cfg.bloom_config();
        assert!(!b.is_enabled);
        let s = cfg.ssao_config();
        assert!(s.is_enabled);
        assert_eq!(s.kernel_size, 12);
    }

    #[test]
    fn default_postfx_stack_matches_default_config() {
        assert_eq!(PostStackConfig::default().enable_ssao, DEFAULT_POSTFX_STACK.enable_ssao);
        assert_eq!(PostStackConfig::default().quality, DEFAULT_POSTFX_STACK.quality);
    }
}
