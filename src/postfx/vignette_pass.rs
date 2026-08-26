// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 KooshaPari <kooshapari@gmail.com>

//! `VignettePass` — vignette darkening at screen edges via the `IPostFxPass`
//! hexagonal port.
//!
//! Mirrors the C# `VignettePass.cs` in `phenotype-postfx` (L5-112 port).
//! Applies a radial darkening effect that fades from the center outward,
//! configurable via intensity and smoothness parameters.

use serde::{Deserialize, Serialize};

use crate::postfx::error::{PostFxError, PostFxResult};
use crate::postfx::ports::post_fx_pass::{PassDescriptor, PassEffect, PostFxContext, PostFxPass};
use crate::postfx::ports::shader_availability::PostFxShaderAvailability;

/// Stable shader name used by the Vignette pass.
pub const VIGNETTE_SHADER_NAME: &str = "Hidden/Phenotype/VignettePass";
/// Required shader keyword for the Vignette variant.
pub const VIGNETTE_KEYWORD: &str = "VIGNETTE";

/// Configuration for the vignette pass.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VignetteConfig {
    /// Whether the pass is enabled.
    pub is_enabled: bool,
    /// Strength of the darkening effect (0.0 = none, 1.0 = full black edges).
    pub intensity: f32,
    /// Smoothness of the vignette falloff (higher = harder edge).
    pub smoothness: f32,
    /// Center point in normalized coordinates `[0.0..1.0, 0.0..1.0]`.
    pub center: [f32; 2],
    /// Roundness of the vignette shape (0.0 = rectangular, 1.0 = circular).
    pub roundness: f32,
}

impl Default for VignetteConfig {
    fn default() -> Self {
        Self {
            is_enabled: false,
            intensity: 0.45,
            smoothness: 0.6,
            center: [0.5, 0.5],
            roundness: 1.0,
        }
    }
}

impl VignetteConfig {
    /// Returns the static descriptor for this pass (used by
    /// `PostStack::describe_passes`).
    pub fn descriptor() -> PassDescriptor {
        PassDescriptor {
            effect: PassEffect::Vignette,
            shader_name: VIGNETTE_SHADER_NAME.into(),
            default_enabled: false,
            cost: 0.05,
            high_keyword: VIGNETTE_KEYWORD.into(),
            description: "Vignette darkening (radial falloff from center).".into(),
        }
    }

    /// Compute the vignette attenuation factor for a given UV coordinate.
    ///
    /// Returns a value in `[0.0, 1.0]` where 1.0 = full brightness (center)
    /// and 0.0 = fully darkened (edge).
    pub fn vignette_factor(&self, u: f32, v: f32) -> f32 {
        let dx = u - self.center[0];
        let dy = v - self.center[1];
        // Approximate elliptical distance with roundness parameter
        let dist_sq = dx * dx + self.roundness * dy * dy;
        let dist = dist_sq.sqrt();
        // Smoothstep-style falloff: 0.0 at center, rising to 1.0 at edges
        let falloff = (dist * self.smoothness * 2.0).clamp(0.0, 1.0);
        1.0 - falloff * self.intensity
    }

    /// Apply vignette to an RGB pixel at the given UV coordinate.
    pub fn apply_pixel(&self, r: f32, g: f32, b: f32, u: f32, v: f32) -> (f32, f32, f32) {
        let f = self.vignette_factor(u, v);
        (r * f, g * f, b * f)
    }
}

/// Adapter that applies a [`VignetteConfig`] to the BRP pass surface.
pub struct VignettePass {
    config: VignetteConfig,
}

impl VignettePass {
    /// New vignette pass with the given config.
    pub fn new(config: VignetteConfig) -> Self {
        Self { config }
    }

    /// Borrow the current config.
    pub fn config(&self) -> &VignetteConfig {
        &self.config
    }

    /// Mutably borrow the current config.
    pub fn config_mut(&mut self) -> &mut VignetteConfig {
        &mut self.config
    }
}

impl PostFxPass for VignettePass {
    fn name(&self) -> &str {
        "Vignette"
    }
    fn effect(&self) -> PassEffect {
        PassEffect::Vignette
    }
    fn cost(&self) -> f32 {
        0.05
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
        if !provider.is_available(VIGNETTE_SHADER_NAME, VIGNETTE_KEYWORD) {
            return Err(PostFxError::ShaderVariantUnavailable {
                shader_name: VIGNETTE_SHADER_NAME.into(),
                keyword: VIGNETTE_KEYWORD.into(),
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
        let c = VignetteConfig::default();
        assert!(!c.is_enabled);
        assert!((c.intensity - 0.45).abs() < f32::EPSILON);
        assert!((c.smoothness - 0.6).abs() < f32::EPSILON);
        assert_eq!(c.center, [0.5, 0.5]);
        assert!((c.roundness - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn descriptor_is_stable() {
        let d = VignetteConfig::descriptor();
        assert_eq!(d.effect, PassEffect::Vignette);
        assert_eq!(d.shader_name, "Hidden/Phenotype/VignettePass");
        assert!(!d.default_enabled);
    }

    #[test]
    fn vignette_factor_at_center_is_full_brightness() {
        let cfg = VignetteConfig::default();
        let f = cfg.vignette_factor(0.5, 0.5);
        assert!((f - 1.0).abs() < 0.01, "center factor={f}");
    }

    #[test]
    fn vignette_factor_at_corner_is_darkened() {
        let cfg = VignetteConfig {
            intensity: 1.0,
            smoothness: 1.0,
            ..VignetteConfig::default()
        };
        let f = cfg.vignette_factor(0.0, 0.0);
        assert!(f < 1.0, "corner factor={f} expected < 1.0");
    }

    #[test]
    fn apply_pixel_preserves_color_ratio() {
        let cfg = VignetteConfig::default();
        let (r, g, b) = cfg.apply_pixel(1.0, 0.5, 0.25, 0.5, 0.5);
        // At center, factor is ~1.0, so ratio is preserved
        assert!((r - 1.0).abs() < 0.01);
        assert!((g - 0.5).abs() < 0.01);
        assert!((b - 0.25).abs() < 0.01);
    }

    #[test]
    fn validate_variants_passes_with_default() {
        let pass = VignettePass::new(VignetteConfig::default());
        let provider = DefaultPostFxShaderAvailability;
        assert!(pass.validate_variants(&provider).is_ok());
    }

    #[test]
    fn validate_variants_fails_when_unavailable() {
        use crate::postfx::ports::shader_availability::MapPostFxShaderAvailability;
        let mut provider = MapPostFxShaderAvailability::new();
        provider.set(VIGNETTE_SHADER_NAME, VIGNETTE_KEYWORD, false);
        let pass = VignettePass::new(VignetteConfig::default());
        assert!(pass.validate_variants(&provider).is_err());
    }

    #[test]
    fn trait_surface_works() {
        let mut pass = VignettePass::new(VignetteConfig::default());
        assert_eq!(pass.name(), "Vignette");
        assert_eq!(pass.effect(), PassEffect::Vignette);
        assert!(!pass.is_enabled());
        pass.set_enabled(true);
        assert!(pass.is_enabled());
        let ctx = PostFxContext::new(0, 0, 0, PassQuality::High);
        assert!(pass.on_setup(&ctx).is_ok());
        assert!(pass.on_render(&ctx).is_ok());
        pass.on_dispose();
    }
}
