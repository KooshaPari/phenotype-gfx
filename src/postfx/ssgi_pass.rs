// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 KooshaPari <kooshapari@gmail.com>

//! `SsgiPass` — Screen-Space Global Illumination via the `IPostFxPass`
//! hexagonal port.
//!
//! Mirrors the C# `SSGIPass.cs` in `phenotype-postfx` (L5-112 port).
//! Implements a simplified screen-space GI approximation using randomized
//! sampling with configurable sample count and radius.

use serde::{Deserialize, Serialize};

use crate::postfx::error::{PostFxError, PostFxResult};
use crate::postfx::ports::post_fx_pass::{PassDescriptor, PassEffect, PostFxContext, PostFxPass};
use crate::postfx::ports::shader_availability::PostFxShaderAvailability;

/// Stable shader name used by the SSGI pass.
pub const SSGI_SHADER_NAME: &str = "Hidden/Phenotype/SSGIPass";
/// Required shader keyword for the SSGI variant.
pub const SSGI_KEYWORD: &str = "SSGIPASS";

/// Configuration for the SSGI pass.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SsgiConfig {
    /// Whether the pass is enabled.
    pub is_enabled: bool,
    /// Number of GI samples per pixel.
    pub samples: u32,
    /// World-space radius of the GI sampling hemisphere.
    pub radius: f32,
    /// Intensity multiplier for the GI contribution.
    pub intensity: f32,
}

impl Default for SsgiConfig {
    fn default() -> Self {
        Self {
            is_enabled: false,
            samples: 12,
            radius: 1.8,
            intensity: 0.45,
        }
    }
}

impl SsgiConfig {
    /// Returns the static descriptor for this pass (used by
    /// `PostStack::describe_passes`).
    pub fn descriptor() -> PassDescriptor {
        PassDescriptor {
            effect: PassEffect::Ssgi,
            shader_name: SSGI_SHADER_NAME.into(),
            default_enabled: false,
            cost: 0.30,
            high_keyword: SSGI_KEYWORD.into(),
            description: "Screen-space global illumination (hemisphere sampling).".into(),
        }
    }

    /// Compute a deterministic pseudo-random sample direction from an index.
    ///
    /// Returns `(offset_x, offset_y, weight)` where the offset is in
    /// `[-1, 1]` range and weight is in `[0, 1]`.
    pub fn sample_direction(index: u32, total: u32) -> (f32, f32, f32) {
        // Golden-ratio based low-discrepancy sequence
        let phi = std::f32::consts::PI * (1.0 + 5.0_f32.sqrt());
        let u = ((index as f32 + 0.5) / total as f32) % 1.0;
        let v = (index as f32 * phi) % 1.0;
        let x = u * 2.0 - 1.0;
        let y = v * 2.0 - 1.0;
        let weight = 1.0 / (1.0 + (x * x + y * y));
        (x, y, weight)
    }

    /// Approximate GI contribution from a set of sample directions.
    pub fn compute_gi_contribution(&self) -> f32 {
        let mut total_weight = 0.0_f32;
        for i in 0..self.samples {
            let (_x, _y, weight) = Self::sample_direction(i, self.samples);
            total_weight += weight;
        }
        // Normalize to [0, 1] range and apply intensity
        let normalized = total_weight / self.samples as f32;
        (normalized * self.intensity).clamp(0.0, 1.0)
    }
}

/// Adapter that applies an [`SsgiConfig`] to the BRP pass surface.
pub struct SsgiPass {
    config: SsgiConfig,
}

impl SsgiPass {
    /// New SSGI pass with the given config.
    pub fn new(config: SsgiConfig) -> Self {
        Self { config }
    }

    /// Borrow the current config.
    pub fn config(&self) -> &SsgiConfig {
        &self.config
    }

    /// Mutably borrow the current config.
    pub fn config_mut(&mut self) -> &mut SsgiConfig {
        &mut self.config
    }
}

impl PostFxPass for SsgiPass {
    fn name(&self) -> &str {
        "SSGI"
    }
    fn effect(&self) -> PassEffect {
        PassEffect::Ssgi
    }
    fn cost(&self) -> f32 {
        0.30
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
        if !provider.is_available(SSGI_SHADER_NAME, SSGI_KEYWORD) {
            return Err(PostFxError::ShaderVariantUnavailable {
                shader_name: SSGI_SHADER_NAME.into(),
                keyword: SSGI_KEYWORD.into(),
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
        let c = SsgiConfig::default();
        assert!(!c.is_enabled);
        assert_eq!(c.samples, 12);
        assert!((c.radius - 1.8).abs() < f32::EPSILON);
        assert!((c.intensity - 0.45).abs() < f32::EPSILON);
    }

    #[test]
    fn descriptor_is_stable() {
        let d = SsgiConfig::descriptor();
        assert_eq!(d.effect, PassEffect::Ssgi);
        assert_eq!(d.shader_name, "Hidden/Phenotype/SSGIPass");
        assert!(!d.default_enabled);
    }

    #[test]
    fn sample_direction_deterministic() {
        let (x1, y1, w1) = SsgiConfig::sample_direction(0, 8);
        let (x2, y2, w2) = SsgiConfig::sample_direction(0, 8);
        assert!((x1 - x2).abs() < f32::EPSILON);
        assert!((y1 - y2).abs() < f32::EPSILON);
        assert!((w1 - w2).abs() < f32::EPSILON);
    }

    #[test]
    fn sample_direction_range() {
        for i in 0..16 {
            let (x, y, w) = SsgiConfig::sample_direction(i, 16);
            assert!(x >= -1.0 && x <= 1.0, "x={x}");
            assert!(y >= -1.0 && y <= 1.0, "y={y}");
            assert!(w > 0.0, "w={w}");
        }
    }

    #[test]
    fn compute_gi_contribution_bounded() {
        let cfg = SsgiConfig {
            samples: 24,
            intensity: 0.6,
            ..SsgiConfig::default()
        };
        let gi = cfg.compute_gi_contribution();
        assert!(gi >= 0.0 && gi <= 1.0, "gi={gi}");
    }

    #[test]
    fn validate_variants_passes_with_default() {
        let pass = SsgiPass::new(SsgiConfig::default());
        let provider = DefaultPostFxShaderAvailability;
        assert!(pass.validate_variants(&provider).is_ok());
    }

    #[test]
    fn validate_variants_fails_when_unavailable() {
        use crate::postfx::ports::shader_availability::MapPostFxShaderAvailability;
        let mut provider = MapPostFxShaderAvailability::new();
        provider.set(SSGI_SHADER_NAME, SSGI_KEYWORD, false);
        let pass = SsgiPass::new(SsgiConfig::default());
        assert!(pass.validate_variants(&provider).is_err());
    }

    #[test]
    fn trait_surface_works() {
        let mut pass = SsgiPass::new(SsgiConfig::default());
        assert_eq!(pass.name(), "SSGI");
        assert_eq!(pass.effect(), PassEffect::Ssgi);
        assert!(!pass.is_enabled());
        pass.set_enabled(true);
        assert!(pass.is_enabled());
        let ctx = PostFxContext::new(0, 0, 0, PassQuality::High);
        assert!(pass.on_setup(&ctx).is_ok());
        assert!(pass.on_render(&ctx).is_ok());
        pass.on_dispose();
    }
}
