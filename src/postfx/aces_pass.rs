// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 KooshaPari <kooshapari@gmail.com>

//! `AcesPass` — ACES filmic tone mapping via the `IPostFxPass` hexagonal port.
//!
//! Mirrors the C# `AcesPass.cs` in `phenotype-postfx` (L5-112 port).
//! Implements a simplified ACES filmic curve approximation for HDR-to-LDR
//! mapping, configurable via exposure and gamma parameters.

use serde::{Deserialize, Serialize};

use crate::postfx::error::{PostFxError, PostFxResult};
use crate::postfx::ports::post_fx_pass::{PassDescriptor, PassEffect, PostFxContext, PostFxPass};
use crate::postfx::ports::shader_availability::PostFxShaderAvailability;

/// Stable shader name used by the ACES pass.
pub const ACES_SHADER_NAME: &str = "Hidden/Phenotype/ACESPass";
/// Required shader keyword for the ACES variant.
pub const ACES_KEYWORD: &str = "ACES";

/// Configuration for the ACES tone mapping pass.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AcesConfig {
    /// Whether the pass is enabled.
    pub is_enabled: bool,
    /// Exposure multiplier applied before tone mapping.
    pub exposure: f32,
    /// Gamma correction exponent applied after tone mapping.
    pub gamma: f32,
}

impl Default for AcesConfig {
    fn default() -> Self {
        Self {
            is_enabled: true,
            exposure: 1.0,
            gamma: 2.2,
        }
    }
}

impl AcesConfig {
    /// Returns the static descriptor for this pass (used by
    /// `PostStack::describe_passes`).
    pub fn descriptor() -> PassDescriptor {
        PassDescriptor {
            effect: PassEffect::Aces,
            shader_name: ACES_SHADER_NAME.into(),
            default_enabled: true,
            cost: 0.10,
            high_keyword: ACES_KEYWORD.into(),
            description: "ACES filmic tone mapping (HDR to LDR).".into(),
        }
    }

    /// Apply simplified ACES filmic curve to a single channel value.
    ///
    /// The approximation uses the standard ACES fit coefficients from
    /// <https://knarkowicz.wordpress.com/2016/01/06/aces-filmic-tone-mapping-curve/>.
    pub fn aces_filmic(x: f32) -> f32 {
        let a = 2.51;
        let b = 0.03;
        let c = 2.43;
        let d = 0.59;
        let e = 0.14;
        let numerator = x * (a * x + b);
        let denominator = x * (c * x + d) + e;
        (numerator / denominator).clamp(0.0, 1.0)
    }

    /// Process an RGB pixel triplet through exposure and ACES tone mapping.
    pub fn process_pixel(&self, r: f32, g: f32, b: f32) -> (f32, f32, f32) {
        let inv_gamma = 1.0 / self.gamma;
        let r = Self::aces_filmic(r * self.exposure).powf(inv_gamma);
        let g = Self::aces_filmic(g * self.exposure).powf(inv_gamma);
        let b = Self::aces_filmic(b * self.exposure).powf(inv_gamma);
        (r, g, b)
    }
}

/// Adapter that applies an [`AcesConfig`] to the BRP pass surface.
pub struct AcesPass {
    config: AcesConfig,
}

impl AcesPass {
    /// New ACES pass with the given config.
    pub fn new(config: AcesConfig) -> Self {
        Self { config }
    }

    /// Borrow the current config.
    pub fn config(&self) -> &AcesConfig {
        &self.config
    }

    /// Mutably borrow the current config.
    pub fn config_mut(&mut self) -> &mut AcesConfig {
        &mut self.config
    }
}

impl PostFxPass for AcesPass {
    fn name(&self) -> &str {
        "ACES"
    }
    fn effect(&self) -> PassEffect {
        PassEffect::Aces
    }
    fn cost(&self) -> f32 {
        0.10
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
    fn validate_variants(
        &self,
        provider: &dyn PostFxShaderAvailability,
    ) -> Result<(), PostFxError> {
        if !provider.is_available(ACES_SHADER_NAME, ACES_KEYWORD) {
            return Err(PostFxError::ShaderVariantUnavailable {
                shader_name: ACES_SHADER_NAME.into(),
                keyword: ACES_KEYWORD.into(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::postfx::ports::post_fx_pass::{PassEffect, PassQuality, PostFxContext};
    use crate::postfx::ports::shader_availability::DefaultPostFxShaderAvailability;

    #[test]
    fn default_config() {
        let c = AcesConfig::default();
        assert!(c.is_enabled);
        assert!((c.exposure - 1.0).abs() < f32::EPSILON);
        assert!((c.gamma - 2.2).abs() < 0.001);
    }

    #[test]
    fn descriptor_is_stable() {
        let d = AcesConfig::descriptor();
        assert_eq!(d.effect, PassEffect::Aces);
        assert_eq!(d.shader_name, "Hidden/Phenotype/ACESPass");
        assert!(d.default_enabled);
    }

    #[test]
    fn aces_filmic_maps_zero_to_zero() {
        let v = AcesConfig::aces_filmic(0.0);
        assert!((v).abs() < f32::EPSILON);
    }

    #[test]
    fn aces_filmic_maps_one_to_near_one() {
        let v = AcesConfig::aces_filmic(1.0);
        assert!(v > 0.7 && v <= 1.0, "expected near 1.0, got {v}");
    }

    #[test]
    fn process_pixel_applies_exposure_and_gamma() {
        let cfg = AcesConfig {
            exposure: 2.0,
            gamma: 2.2,
            ..AcesConfig::default()
        };
        let (r, g, b) = cfg.process_pixel(0.5, 0.5, 0.5);
        // exposure doubles input, ACES compresses, then gamma expands
        assert!(r > 0.0 && r <= 1.0);
        assert!((r - g).abs() < f32::EPSILON);
        assert!((g - b).abs() < f32::EPSILON);
    }

    #[test]
    fn validate_variants_passes_with_default() {
        let pass = AcesPass::new(AcesConfig::default());
        let provider = DefaultPostFxShaderAvailability;
        assert!(pass.validate_variants(&provider).is_ok());
    }

    #[test]
    fn validate_variants_fails_when_unavailable() {
        use crate::postfx::ports::shader_availability::MapPostFxShaderAvailability;
        let mut provider = MapPostFxShaderAvailability::new();
        provider.set(ACES_SHADER_NAME, ACES_KEYWORD, false);
        let pass = AcesPass::new(AcesConfig::default());
        assert!(pass.validate_variants(&provider).is_err());
    }

    #[test]
    fn trait_surface_works() {
        let mut pass = AcesPass::new(AcesConfig::default());
        assert_eq!(pass.name(), "ACES");
        assert_eq!(pass.effect(), PassEffect::Aces);
        assert!(pass.is_enabled());
        pass.set_enabled(false);
        assert!(!pass.is_enabled());
        let ctx = PostFxContext::new(0, 0, 0, PassQuality::High);
        assert!(pass.on_setup(&ctx).is_ok());
        assert!(pass.on_render(&ctx).is_ok());
        pass.on_dispose();
    }
}
