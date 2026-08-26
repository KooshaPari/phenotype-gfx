// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 KooshaPari <kooshapari@gmail.com>

//! `LutPass` — LUT (Look-Up Table) color grading via the `IPostFxPass`
//! hexagonal port.
//!
//! Mirrors the C# `LutPass.cs` in `phenotype-postfx` (L5-112 port).
//! Applies a 3D color grading LUT lookup, configurable via LUT size and
//! intensity. The LUT is applied as a linear blend between the original
//! and graded color.

use serde::{Deserialize, Serialize};

use crate::postfx::error::{PostFxError, PostFxResult};
use crate::postfx::ports::post_fx_pass::{PassDescriptor, PassEffect, PostFxContext, PostFxPass};
use crate::postfx::ports::shader_availability::PostFxShaderAvailability;

/// Stable shader name used by the LUT pass.
pub const LUT_SHADER_NAME: &str = "Hidden/Phenotype/LutPass";
/// Required shader keyword for the LUT variant.
pub const LUT_KEYWORD: &str = "LUT";

/// Configuration for the LUT color grading pass.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LutConfig {
    /// Whether the pass is enabled.
    pub is_enabled: bool,
    /// Number of entries per axis in the 3D LUT (e.g. 16 for a 16x16x16 LUT).
    pub lut_size: u32,
    /// Blend factor between original and graded color (0.0 = original, 1.0 = fully graded).
    pub intensity: f32,
}

impl Default for LutConfig {
    fn default() -> Self {
        Self {
            is_enabled: true,
            lut_size: 16,
            intensity: 1.0,
        }
    }
}

impl LutConfig {
    /// Returns the static descriptor for this pass (used by
    /// `PostStack::describe_passes`).
    pub fn descriptor() -> PassDescriptor {
        PassDescriptor {
            effect: PassEffect::Lut,
            shader_name: LUT_SHADER_NAME.into(),
            default_enabled: true,
            cost: 0.12,
            high_keyword: LUT_KEYWORD.into(),
            description: "LUT color grading (3D lookup table blend).".into(),
        }
    }

    /// Convert an RGB color to a 3D LUT index triplet.
    ///
    /// Each channel is clamped to `[0, 1]` and mapped to `[0, lut_size-1]`.
    pub fn rgb_to_lut_index(&self, r: f32, g: f32, b: f32) -> (u32, u32, u32) {
        let max_idx = self.lut_size.saturating_sub(1) as f32;
        let ri = (r.clamp(0.0, 1.0) * max_idx).round() as u32;
        let gi = (g.clamp(0.0, 1.0) * max_idx).round() as u32;
        let bi = (b.clamp(0.0, 1.0) * max_idx).round() as u32;
        (ri, gi, bi)
    }

    /// Compute a flat index into a 1D array representing a 3D LUT.
    pub fn flat_index(&self, ri: u32, gi: u32, bi: u32) -> usize {
        (bi * self.lut_size * self.lut_size + gi * self.lut_size + ri) as usize
    }

    /// Apply LUT color grading to an RGB pixel using a pre-computed LUT.
    ///
    /// The `lut` slice should have `lut_size^3 * 3` entries (RGB triplets).
    /// Returns the graded pixel blended by `intensity`.
    pub fn apply_pixel(&self, r: f32, g: f32, b: f32, lut: &[f32]) -> (f32, f32, f32) {
        let total = self.lut_size * self.lut_size * self.lut_size * 3;
        if lut.len() < total as usize {
            return (r, g, b);
        }

        let (ri, gi, bi) = self.rgb_to_lut_index(r, g, b);
        let idx = self.flat_index(ri, gi, bi) * 3;
        let lr = lut[idx];
        let lg = lut[idx + 1];
        let lb = lut[idx + 2];

        // Linear blend between original and LUT-graded
        let t = self.intensity;
        let out_r = r + (lr - r) * t;
        let out_g = g + (lg - g) * t;
        let out_b = b + (lb - b) * t;
        (out_r, out_g, out_b)
    }

    /// Generate an identity LUT (maps each color to itself).
    pub fn identity_lut(&self) -> Vec<f32> {
        let n = self.lut_size;
        let total = n * n * n * 3;
        let mut lut = Vec::with_capacity(total as usize);
        let max_idx = n.saturating_sub(1) as f32;
        for b in 0..n {
            for g in 0..n {
                for r in 0..n {
                    lut.push(r as f32 / max_idx);
                    lut.push(g as f32 / max_idx);
                    lut.push(b as f32 / max_idx);
                }
            }
        }
        lut
    }
}

/// Adapter that applies a [`LutConfig`] to the BRP pass surface.
pub struct LutPass {
    config: LutConfig,
    lut: Vec<f32>,
}

impl LutPass {
    /// New LUT pass with the given config and an identity LUT.
    pub fn new(config: LutConfig) -> Self {
        let lut = config.identity_lut();
        Self { config, lut }
    }

    /// New LUT pass with a custom LUT.
    pub fn with_lut(config: LutConfig, lut: Vec<f32>) -> Self {
        Self { config, lut }
    }

    /// Borrow the current config.
    pub fn config(&self) -> &LutConfig {
        &self.config
    }

    /// Mutably borrow the current config.
    pub fn config_mut(&mut self) -> &mut LutConfig {
        &mut self.config
    }

    /// Borrow the LUT data.
    pub fn lut(&self) -> &[f32] {
        &self.lut
    }

    /// Replace the LUT data.
    pub fn set_lut(&mut self, lut: Vec<f32>) {
        self.lut = lut;
    }
}

impl PostFxPass for LutPass {
    fn name(&self) -> &str {
        "LUT"
    }
    fn effect(&self) -> PassEffect {
        PassEffect::Lut
    }
    fn cost(&self) -> f32 {
        0.12
    }
    fn is_enabled(&self) -> bool {
        self.config.is_enabled
    }
    fn set_enabled(&mut self, e: bool) {
        self.config.is_enabled = e;
    }
    fn on_setup(&mut self, _ctx: &PostFxContext) -> PostFxResult<()> {
        if self.lut.is_empty() {
            self.lut = self.config.identity_lut();
        }
        Ok(())
    }
    fn on_render(&mut self, _ctx: &PostFxContext) -> PostFxResult<()> {
        Ok(())
    }
    fn on_dispose(&mut self) {
        self.lut.clear();
    }
    fn validate_variants(
        &self,
        provider: &dyn PostFxShaderAvailability,
    ) -> Result<(), PostFxError> {
        if !provider.is_available(LUT_SHADER_NAME, LUT_KEYWORD) {
            return Err(PostFxError::ShaderVariantUnavailable {
                shader_name: LUT_SHADER_NAME.into(),
                keyword: LUT_KEYWORD.into(),
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
        let c = LutConfig::default();
        assert!(c.is_enabled);
        assert_eq!(c.lut_size, 16);
        assert!((c.intensity - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn descriptor_is_stable() {
        let d = LutConfig::descriptor();
        assert_eq!(d.effect, PassEffect::Lut);
        assert_eq!(d.shader_name, "Hidden/Phenotype/LutPass");
        assert!(d.default_enabled);
    }

    #[test]
    fn identity_lut_size() {
        let cfg = LutConfig { lut_size: 8, ..LutConfig::default() };
        let lut = cfg.identity_lut();
        assert_eq!(lut.len(), 8 * 8 * 8 * 3);
    }

    #[test]
    fn identity_lut_preserves_color() {
        let cfg = LutConfig { lut_size: 8, ..LutConfig::default() };
        let lut = cfg.identity_lut();
        let (r, g, b) = cfg.apply_pixel(0.5, 0.5, 0.5, &lut);
        assert!((r - 0.5).abs() < 0.1, "r={r}");
        assert!((g - 0.5).abs() < 0.1, "g={g}");
        assert!((b - 0.5).abs() < 0.1, "b={b}");
    }

    #[test]
    fn apply_pixel_clamps_to_identity_when_lut_empty() {
        let cfg = LutConfig::default();
        let (r, g, b) = cfg.apply_pixel(0.8, 0.6, 0.4, &[]);
        // Returns original pixel when LUT is too small
        assert!((r - 0.8).abs() < f32::EPSILON);
        assert!((g - 0.6).abs() < f32::EPSILON);
        assert!((b - 0.4).abs() < f32::EPSILON);
    }

    #[test]
    fn flat_index_is_unique() {
        let cfg = LutConfig { lut_size: 4, ..LutConfig::default() };
        let mut indices = std::collections::HashSet::new();
        for b in 0..4u32 {
            for g in 0..4u32 {
                for r in 0..4u32 {
                    let idx = cfg.flat_index(r, g, b);
                    assert!(indices.insert(idx), "duplicate index {idx}");
                }
            }
        }
    }

    #[test]
    fn validate_variants_passes_with_default() {
        let pass = LutPass::new(LutConfig::default());
        let provider = DefaultPostFxShaderAvailability;
        assert!(pass.validate_variants(&provider).is_ok());
    }

    #[test]
    fn validate_variants_fails_when_unavailable() {
        use crate::postfx::ports::shader_availability::MapPostFxShaderAvailability;
        let mut provider = MapPostFxShaderAvailability::new();
        provider.set(LUT_SHADER_NAME, LUT_KEYWORD, false);
        let pass = LutPass::new(LutConfig::default());
        assert!(pass.validate_variants(&provider).is_err());
    }

    #[test]
    fn trait_surface_works() {
        let mut pass = LutPass::new(LutConfig::default());
        assert_eq!(pass.name(), "LUT");
        assert_eq!(pass.effect(), PassEffect::Lut);
        assert!(pass.is_enabled());
        pass.set_enabled(false);
        assert!(!pass.is_enabled());
        let ctx = PostFxContext::new(0, 0, 0, PassQuality::High);
        assert!(pass.on_setup(&ctx).is_ok());
        assert!(pass.on_render(&ctx).is_ok());
        pass.on_dispose();
        assert!(pass.lut().is_empty());
    }
}
