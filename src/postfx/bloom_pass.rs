// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 KooshaPari <kooshapari@gmail.com>

//! `BloomPass` — multi-pass bloom configuration.
//!
//! Mirrors the C# `BloomPass.cs` in `phenotype-postfx` (L5-112 port).
//! Configurable threshold, intensity, and iteration count; selects a
//! shader quality variant (Low / Medium / High / Ultra) based on the
//! current [`PassQuality`].

use serde::{Deserialize, Serialize};

use crate::postfx::error::{PostFxError, PostFxResult};
use crate::postfx::ports::post_fx_pass::{
    PassDescriptor, PassEffect, PassQuality, PostFxContext, PostFxPass,
};
use crate::postfx::ports::shader_availability::PostFxShaderAvailability;

/// Stable shader name used by the bloom pass.
pub const BLOOM_SHADER_NAME: &str = "Hidden/Phenotype/BloomPass";

/// Quality-keyword constants used to select shader variants.
pub const BLOOM_LOW_KEYWORD: &str = "BLOOM_LOW";
pub const BLOOM_MEDIUM_KEYWORD: &str = "BLOOM_MEDIUM";
pub const BLOOM_HIGH_KEYWORD: &str = "BLOOM_HIGH";
pub const BLOOM_ULTRA_KEYWORD: &str = "BLOOM_ULTRA";

/// Configuration for the bloom pass.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BloomConfig {
    /// Whether the pass is enabled.
    pub is_enabled: bool,
    /// Luminance threshold above which pixels contribute to bloom.
    pub threshold: f32,
    /// Intensity multiplier for the final bloom composite.
    pub intensity: f32,
    /// Number of iterative blur passes.
    pub iterations: u32,
}

impl Default for BloomConfig {
    fn default() -> Self {
        Self {
            is_enabled: true,
            threshold: 0.8,
            intensity: 0.5,
            iterations: 2,
        }
    }
}

impl BloomConfig {
    /// Returns the static descriptor for this pass (used by
    /// `PostStack::describe_passes`).
    pub fn descriptor() -> PassDescriptor {
        PassDescriptor {
            effect: PassEffect::Bloom,
            shader_name: BLOOM_SHADER_NAME.into(),
            default_enabled: true,
            cost: 0.35,
            high_keyword: BLOOM_HIGH_KEYWORD.into(),
            description: "Multi-pass bloom (threshold -> blur H -> blur V -> composite).".into(),
        }
    }

    /// Returns the quality keyword to enable for the given quality level.
    pub fn quality_keyword(quality: PassQuality) -> &'static str {
        match quality {
            PassQuality::Off => BLOOM_MEDIUM_KEYWORD,
            PassQuality::Low => BLOOM_LOW_KEYWORD,
            PassQuality::Medium => BLOOM_MEDIUM_KEYWORD,
            PassQuality::High => BLOOM_HIGH_KEYWORD,
            PassQuality::Ultra => BLOOM_ULTRA_KEYWORD,
        }
    }
}

/// Adapter that applies a [`BloomConfig`] to the BRP pass surface.
pub struct BloomPass {
    config: BloomConfig,
}

impl BloomPass {
    /// New bloom pass with the given config.
    pub fn new(config: BloomConfig) -> Self {
        Self { config }
    }

    /// Borrow the current config.
    pub fn config(&self) -> &BloomConfig {
        &self.config
    }

    /// Mutably borrow the current config.
    pub fn config_mut(&mut self) -> &mut BloomConfig {
        &mut self.config
    }

    /// Number of blits performed in one render call: 1 (threshold) +
    /// `iterations * 2` (blur H + blur V) + 1 (composite).
    pub fn blit_count(&self) -> u32 {
        1 + self.config.iterations * 2 + 1
    }
}

impl PostFxPass for BloomPass {
    fn name(&self) -> &str {
        "Bloom"
    }
    fn effect(&self) -> PassEffect {
        PassEffect::Bloom
    }
    fn cost(&self) -> f32 {
        0.35
    }
    fn is_enabled(&self) -> bool {
        self.config.is_enabled
    }
    fn set_enabled(&mut self, e: bool) {
        self.config.is_enabled = e;
    }
    fn on_setup(&mut self, _ctx: &PostFxContext) -> PostFxResult<()> {
        Ok(())
    }
    fn on_render(&mut self, _ctx: &PostFxContext) -> PostFxResult<()> {
        Ok(())
    }
    fn on_dispose(&mut self) {}
    fn validate_variants(&self, provider: &dyn PostFxShaderAvailability) -> Result<(), PostFxError> {
        let keyword = BloomConfig::quality_keyword(PassQuality::High);
        if !provider.is_available(BLOOM_SHADER_NAME, keyword) {
            return Err(PostFxError::ShaderVariantUnavailable {
                shader_name: BLOOM_SHADER_NAME.into(),
                keyword: keyword.into(),
            });
        }
        Ok(())
    }
}

// `PassContext` is a re-export of `PostFxContext` for ergonomics in the
// `describe_passes` API; the legacy C# code used the name `PostFxContext`
// (which we keep) and `PassContext` is exposed here for symmetry with the
// future per-pass context work.
#[allow(dead_code)]
pub type PassContext = PostFxContext;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::postfx::ports::shader_availability::DefaultPostFxShaderAvailability;

    #[test]
    fn default_config() {
        let c = BloomConfig::default();
        assert!(c.is_enabled);
        assert!((c.threshold - 0.8).abs() < f32::EPSILON);
        assert!((c.intensity - 0.5).abs() < f32::EPSILON);
        assert_eq!(c.iterations, 2);
    }

    #[test]
    fn quality_keyword_mapping() {
        assert_eq!(BloomConfig::quality_keyword(PassQuality::Low), BLOOM_LOW_KEYWORD);
        assert_eq!(BloomConfig::quality_keyword(PassQuality::Medium), BLOOM_MEDIUM_KEYWORD);
        assert_eq!(BloomConfig::quality_keyword(PassQuality::High), BLOOM_HIGH_KEYWORD);
        assert_eq!(BloomConfig::quality_keyword(PassQuality::Ultra), BLOOM_ULTRA_KEYWORD);
    }

    #[test]
    fn blit_count_scales_with_iterations() {
        let mut cfg = BloomConfig::default();
        cfg.iterations = 3;
        let pass = BloomPass::new(cfg);
        assert_eq!(pass.blit_count(), 8); // 1 + 3*2 + 1
    }

    #[test]
    fn descriptor_is_stable() {
        let d = BloomConfig::descriptor();
        assert_eq!(d.effect, PassEffect::Bloom);
        assert_eq!(d.shader_name, "Hidden/Phenotype/BloomPass");
        assert!(d.default_enabled);
    }

    #[test]
    fn validate_variants_passes_with_default() {
        let pass = BloomPass::new(BloomConfig::default());
        let provider = DefaultPostFxShaderAvailability;
        assert!(pass.validate_variants(&provider).is_ok());
    }

    #[test]
    fn validate_variants_fails_when_unavailable() {
        use crate::postfx::ports::shader_availability::MapPostFxShaderAvailability;
        let mut provider = MapPostFxShaderAvailability::new();
        provider.set(BLOOM_SHADER_NAME, BLOOM_HIGH_KEYWORD, false);
        let pass = BloomPass::new(BloomConfig::default());
        assert!(pass.validate_variants(&provider).is_err());
    }

    #[test]
    fn trait_surface_works() {
        use crate::postfx::ports::post_fx_pass::PostFxContext as Ctx;
        let mut pass = BloomPass::new(BloomConfig::default());
        assert_eq!(pass.name(), "Bloom");
        assert_eq!(pass.effect(), PassEffect::Bloom);
        assert!(pass.is_enabled());
        pass.set_enabled(false);
        assert!(!pass.is_enabled());
        let ctx = Ctx::new(0, 0, 0, PassQuality::High);
        assert!(pass.on_setup(&ctx).is_ok());
        assert!(pass.on_render(&ctx).is_ok());
        pass.on_dispose();
    }
}
