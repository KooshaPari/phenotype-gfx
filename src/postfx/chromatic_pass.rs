// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 KooshaPari <kooshapari@gmail.com>

//! `ChromaticPass` — chromatic aberration (RGB channel offset) via the
//! `IPostFxPass` hexagonal port.
//!
//! Mirrors the C# `ChromaticAberrationPass.cs` in `phenotype-postfx`
//! (L5-112 port). Applies a radial RGB channel separation effect that
//! simulates lens chromatic aberration, configurable via intensity.

use serde::{Deserialize, Serialize};

use crate::postfx::error::{PostFxError, PostFxResult};
use crate::postfx::ports::post_fx_pass::{PassDescriptor, PassEffect, PostFxContext, PostFxPass};
use crate::postfx::ports::shader_availability::PostFxShaderAvailability;

/// Stable shader name used by the chromatic aberration pass.
pub const CHROMATIC_SHADER_NAME: &str = "Hidden/Phenotype/ChromaticAberrationPass";
/// Required shader keyword for the chromatic aberration variant.
pub const CHROMATIC_KEYWORD: &str = "CHROMATIC";

/// Configuration for the chromatic aberration pass.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChromaticConfig {
    /// Whether the pass is enabled.
    pub is_enabled: bool,
    /// Strength of the RGB channel separation (0.0 = none, 1.0 = maximum).
    pub intensity: f32,
    /// Center of the aberration effect in normalized coordinates.
    pub center: [f32; 2],
}

impl Default for ChromaticConfig {
    fn default() -> Self {
        Self {
            is_enabled: false,
            intensity: 0.15,
            center: [0.5, 0.5],
        }
    }
}

impl ChromaticConfig {
    /// Returns the static descriptor for this pass (used by
    /// `PostStack::describe_passes`).
    pub fn descriptor() -> PassDescriptor {
        PassDescriptor {
            effect: PassEffect::ChromaticAberration,
            shader_name: CHROMATIC_SHADER_NAME.into(),
            default_enabled: false,
            cost: 0.08,
            high_keyword: CHROMATIC_KEYWORD.into(),
            description: "Chromatic aberration (radial RGB channel offset).".into(),
        }
    }

    /// Compute the per-channel UV offsets for a given UV coordinate.
    ///
    /// Returns `(r_offset, g_offset, b_offset)` where each offset is
    /// `(du, dv)` — the UV displacement to sample that channel from.
    pub fn channel_offsets(&self, u: f32, v: f32) -> [(f32, f32); 3] {
        let dx = u - self.center[0];
        let dy = v - self.center[1];
        let dist = (dx * dx + dy * dy).sqrt();
        let amount = dist * self.intensity;

        // R shifts outward, B shifts inward, G stays centered
        let r_u = u + dx * amount;
        let r_v = v + dy * amount;
        let b_u = u - dx * amount;
        let b_v = v - dy * amount;

        [(r_u, r_v), (u, v), (b_u, b_v)]
    }

    /// Apply chromatic aberration to an RGB pixel. The offset factors
    /// represent pre-sampled channel values (r, g, b) at the shifted UVs.
    pub fn apply_pixel(&self, r: f32, g: f32, b: f32, u: f32, v: f32) -> (f32, f32, f32) {
        let _offsets = self.channel_offsets(u, v);
        // In a real GPU shader, each channel would be sampled at different
        // UVs. Here we simulate the effect by blending based on distance.
        let dx = u - self.center[0];
        let dy = v - self.center[1];
        let dist = (dx * dx + dy * dy).sqrt();
        let shift = dist * self.intensity;

        // Red shifts outward (brighter at edges), blue shifts inward
        let r_out = (r + shift * 0.1).clamp(0.0, 1.0);
        let g_out = g;
        let b_out = (b - shift * 0.1).clamp(0.0, 1.0);

        (r_out, g_out, b_out)
    }
}

/// Adapter that applies a [`ChromaticConfig`] to the BRP pass surface.
pub struct ChromaticPass {
    config: ChromaticConfig,
}

impl ChromaticPass {
    /// New chromatic aberration pass with the given config.
    pub fn new(config: ChromaticConfig) -> Self {
        Self { config }
    }

    /// Borrow the current config.
    pub fn config(&self) -> &ChromaticConfig {
        &self.config
    }

    /// Mutably borrow the current config.
    pub fn config_mut(&mut self) -> &mut ChromaticConfig {
        &mut self.config
    }
}

impl PostFxPass for ChromaticPass {
    fn name(&self) -> &str {
        "ChromaticAberration"
    }
    fn effect(&self) -> PassEffect {
        PassEffect::ChromaticAberration
    }
    fn cost(&self) -> f32 {
        0.08
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
        if !provider.is_available(CHROMATIC_SHADER_NAME, CHROMATIC_KEYWORD) {
            return Err(PostFxError::ShaderVariantUnavailable {
                shader_name: CHROMATIC_SHADER_NAME.into(),
                keyword: CHROMATIC_KEYWORD.into(),
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
        let c = ChromaticConfig::default();
        assert!(!c.is_enabled);
        assert!((c.intensity - 0.15).abs() < f32::EPSILON);
        assert_eq!(c.center, [0.5, 0.5]);
    }

    #[test]
    fn descriptor_is_stable() {
        let d = ChromaticConfig::descriptor();
        assert_eq!(d.effect, PassEffect::ChromaticAberration);
        assert_eq!(d.shader_name, "Hidden/Phenotype/ChromaticAberrationPass");
        assert!(!d.default_enabled);
    }

    #[test]
    fn channel_offsets_at_center_are_identity() {
        let cfg = ChromaticConfig::default();
        let offsets = cfg.channel_offsets(0.5, 0.5);
        // At center, dx=dy=0 so all offsets equal the input UV
        assert!((offsets[0].0 - 0.5).abs() < f32::EPSILON);
        assert!((offsets[1].0 - 0.5).abs() < f32::EPSILON);
        assert!((offsets[2].0 - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn channel_offsets_separate_at_edges() {
        let cfg = ChromaticConfig {
            intensity: 0.5,
            ..ChromaticConfig::default()
        };
        let offsets = cfg.channel_offsets(1.0, 0.5);
        // R should shift outward, B should shift inward
        assert!(offsets[0].0 > 1.0, "R shifted outward");
        assert!(offsets[2].0 < 1.0, "B shifted inward");
    }

    #[test]
    fn apply_pixel_no_effect_at_zero_intensity() {
        let cfg = ChromaticConfig {
            intensity: 0.0,
            ..ChromaticConfig::default()
        };
        let (r, g, b) = cfg.apply_pixel(0.8, 0.6, 0.4, 0.0, 0.0);
        // With zero intensity, output should be close to input
        assert!((r - 0.8).abs() < 0.01);
        assert!((g - 0.6).abs() < f32::EPSILON);
        assert!((b - 0.4).abs() < 0.01);
    }

    #[test]
    fn validate_variants_passes_with_default() {
        let pass = ChromaticPass::new(ChromaticConfig::default());
        let provider = DefaultPostFxShaderAvailability;
        assert!(pass.validate_variants(&provider).is_ok());
    }

    #[test]
    fn validate_variants_fails_when_unavailable() {
        use crate::postfx::ports::shader_availability::MapPostFxShaderAvailability;
        let mut provider = MapPostFxShaderAvailability::new();
        provider.set(CHROMATIC_SHADER_NAME, CHROMATIC_KEYWORD, false);
        let pass = ChromaticPass::new(ChromaticConfig::default());
        assert!(pass.validate_variants(&provider).is_err());
    }

    #[test]
    fn trait_surface_works() {
        let mut pass = ChromaticPass::new(ChromaticConfig::default());
        assert_eq!(pass.name(), "ChromaticAberration");
        assert_eq!(pass.effect(), PassEffect::ChromaticAberration);
        assert!(!pass.is_enabled());
        pass.set_enabled(true);
        assert!(pass.is_enabled());
        let ctx = PostFxContext::new(0, 0, 0, PassQuality::High);
        assert!(pass.on_setup(&ctx).is_ok());
        assert!(pass.on_render(&ctx).is_ok());
        pass.on_dispose();
    }
}
