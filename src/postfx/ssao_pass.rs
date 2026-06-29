// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 KooshaPari <kooshapari@gmail.com>

//! `SSAOPass` — Screen Space Ambient Occlusion using the `IPostFxPass`
//! hexagonal port.
//!
//! Mirrors the C# `SSAOPass.cs` in `phenotype-postfx` (L5-112 port).
//! Configurable radius, intensity, bias, and kernel size; generates a
//! deterministic sample kernel for depth-buffer sampling.

use serde::{Deserialize, Serialize};

use crate::postfx::error::{PostFxError, PostFxResult};
use crate::postfx::ports::post_fx_pass::{
    PassDescriptor, PassEffect, PassQuality, PostFxContext, PostFxPass,
};
use crate::postfx::ports::shader_availability::PostFxShaderAvailability;

/// Stable shader name used by the SSAO pass.
pub const SSAO_SHADER_NAME: &str = "Hidden/Phenotype/SSAOPass";
/// Required shader keyword for the SSAO variant.
pub const SSAO_KEYWORD: &str = "SSAOPASS";

/// Configuration for the SSAO pass.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SsaoConfig {
    /// Whether the pass is enabled.
    pub is_enabled: bool,
    /// World-space radius of the SSAO sampling sphere.
    pub radius: f32,
    /// Intensity multiplier for the occlusion mask.
    pub intensity: f32,
    /// Depth bias to avoid self-occlusion artifacts.
    pub bias: f32,
    /// Number of samples in the SSAO kernel.
    pub kernel_size: u32,
}

impl Default for SsaoConfig {
    fn default() -> Self {
        Self {
            is_enabled: true,
            radius: 0.5,
            intensity: 1.2,
            bias: 0.04,
            kernel_size: 8,
        }
    }
}

impl SsaoConfig {
    /// Returns the static descriptor for this pass.
    pub fn descriptor() -> PassDescriptor {
        PassDescriptor {
            effect: PassEffect::Ssao,
            shader_name: SSAO_SHADER_NAME.into(),
            default_enabled: true,
            cost: 0.25,
            high_keyword: SSAO_KEYWORD.into(),
            description: "Screen-space ambient occlusion (depth-buffer sampling).".into(),
        }
    }

    /// Generates a deterministic sample kernel of the given size.
    ///
    /// Samples are distributed on a unit disk and scaled so that inner
    /// samples are closer to the origin and outer samples are farther away.
    pub fn build_kernel(size: u32) -> Vec<[f32; 4]> {
        // Deterministic LCG seeded at 1337 (matches the C# `new System.Random(1337)`).
        let mut state: u64 = 1337;
        let mut next = || -> f32 {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((state >> 11) as f64 / (1u64 << 53) as f64) as f32
        };

        let mut kernel = Vec::with_capacity(size as usize);
        for i in 0..size {
            let mut x = lerp(-1.0_f32, 1.0_f32, next());
            let mut y = lerp(-1.0_f32, 1.0_f32, next());
            let mut s = [x, y];
            let sqr = s[0] * s[0] + s[1] * s[1];
            if sqr < 0.0001 {
                s = [1.0, 0.0];
            }
            let inv_len = 1.0 / (s[0] * s[0] + s[1] * s[1]).sqrt();
            s[0] *= inv_len;
            s[1] *= inv_len;
            let scale = (i as f32 + 1.0) / size as f32;
            kernel.push([s[0], s[1], 0.0, scale]);
        }
        kernel
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Adapter that applies an [`SsaoConfig`] to the BRP pass surface.
pub struct SsaoPass {
    config: SsaoConfig,
    kernel: Vec<[f32; 4]>,
}

impl SsaoPass {
    /// New SSAO pass with the given config.
    pub fn new(config: SsaoConfig) -> Self {
        let kernel = SsaoConfig::build_kernel(config.kernel_size);
        Self { config, kernel }
    }

    /// Borrow the current config.
    pub fn config(&self) -> &SsaoConfig {
        &self.config
    }

    /// Mutably borrow the current config.
    pub fn config_mut(&mut self) -> &mut SsaoConfig {
        &mut self.config
    }

    /// Returns the sample kernel. Regenerates if `kernel_size` was changed
    /// without rebuilding.
    pub fn kernel(&self) -> &[[f32; 4]] {
        &self.kernel
    }

    /// Force a kernel rebuild at the current `kernel_size`.
    pub fn rebuild_kernel(&mut self) {
        self.kernel = SsaoConfig::build_kernel(self.config.kernel_size);
    }
}

impl PostFxPass for SsaoPass {
    fn name(&self) -> &str {
        "SSAO"
    }
    fn effect(&self) -> PassEffect {
        PassEffect::Ssao
    }
    fn cost(&self) -> f32 {
        0.25
    }
    fn is_enabled(&self) -> bool {
        self.config.is_enabled
    }
    fn set_enabled(&mut self, e: bool) {
        self.config.is_enabled = e;
    }
    fn on_setup(&mut self, _ctx: &PostFxContext) -> PostFxResult<()> {
        if self.kernel.len() as u32 != self.config.kernel_size {
            self.rebuild_kernel();
        }
        Ok(())
    }
    fn on_render(&mut self, _ctx: &PostFxContext) -> PostFxResult<()> {
        Ok(())
    }
    fn on_dispose(&mut self) {
        self.kernel.clear();
    }
    fn validate_variants(
        &self,
        provider: &dyn PostFxShaderAvailability,
    ) -> Result<(), PostFxError> {
        if !provider.is_available(SSAO_SHADER_NAME, SSAO_KEYWORD) {
            return Err(PostFxError::ShaderVariantUnavailable {
                shader_name: SSAO_SHADER_NAME.into(),
                keyword: SSAO_KEYWORD.into(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::postfx::ports::shader_availability::DefaultPostFxShaderAvailability;

    #[test]
    fn default_config() {
        let c = SsaoConfig::default();
        assert!(c.is_enabled);
        assert!((c.radius - 0.5).abs() < f32::EPSILON);
        assert!((c.intensity - 1.2).abs() < f32::EPSILON);
        assert!((c.bias - 0.04).abs() < f32::EPSILON);
        assert_eq!(c.kernel_size, 8);
    }

    #[test]
    fn build_kernel_correct_size() {
        let k = SsaoConfig::build_kernel(12);
        assert_eq!(k.len(), 12);
    }

    #[test]
    fn build_kernel_respects_size_change() {
        let k1 = SsaoConfig::build_kernel(8);
        let k2 = SsaoConfig::build_kernel(16);
        assert_eq!(k1.len(), 8);
        assert_eq!(k2.len(), 16);
    }

    #[test]
    fn kernel_samples_are_normalized() {
        let k = SsaoConfig::build_kernel(8);
        for s in &k {
            let len = (s[0] * s[0] + s[1] * s[1]).sqrt();
            assert!(len > 0.0);
        }
    }

    #[test]
    fn descriptor_is_stable() {
        let d = SsaoConfig::descriptor();
        assert_eq!(d.effect, PassEffect::Ssao);
        assert_eq!(d.shader_name, "Hidden/Phenotype/SSAOPass");
    }

    #[test]
    fn validate_variants_passes_with_default() {
        let pass = SsaoPass::new(SsaoConfig::default());
        let provider = DefaultPostFxShaderAvailability;
        assert!(pass.validate_variants(&provider).is_ok());
    }

    #[test]
    fn validate_variants_fails_when_unavailable() {
        use crate::postfx::ports::shader_availability::MapPostFxShaderAvailability;
        let mut provider = MapPostFxShaderAvailability::new();
        provider.set(SSAO_SHADER_NAME, SSAO_KEYWORD, false);
        let pass = SsaoPass::new(SsaoConfig::default());
        assert!(pass.validate_variants(&provider).is_err());
    }

    #[test]
    fn trait_surface_works() {
        let mut pass = SsaoPass::new(SsaoConfig::default());
        assert_eq!(pass.name(), "SSAO");
        assert_eq!(pass.effect(), PassEffect::Ssao);
        assert!(pass.is_enabled());
        pass.set_enabled(false);
        assert!(!pass.is_enabled());
        let ctx = PostFxContext::new(0, 0, 0, PassQuality::High);
        assert!(pass.on_setup(&ctx).is_ok());
        assert!(pass.on_render(&ctx).is_ok());
        pass.on_dispose();
        assert!(pass.kernel().is_empty());
    }

    #[test]
    fn rebuild_kernel_resizes() {
        let mut pass = SsaoPass::new(SsaoConfig::default());
        let orig_len = pass.kernel().len();
        pass.config_mut().kernel_size = 24;
        pass.rebuild_kernel();
        assert_eq!(pass.kernel().len(), 24);
        assert_ne!(orig_len, 24);
    }
}
