// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 KooshaPari <kooshapari@gmail.com>

//! `IPostFxPass` — hexagonal port trait for post-processing passes.
//!
//! Every post-fx effect (Bloom, ACES, LUT, SSAO, SSGI, etc.) is an adapter
//! that implements this trait. The `PostStack` driver calls them in order
//! without knowing the concrete type.
//!
//! Reference: `phenotype-voxel/src/ports/*` (T2 SSOT pattern),
//! `phenotype-infra/REUSE.toml` (T20).

use std::fmt;

use crate::postfx::error::{PostFxError, PostFxResult};
use crate::postfx::ports::shader_availability::PostFxShaderAvailability;

/// Identifies each post-processing effect.
///
/// Mirrors the C# `PostFxEffect` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PassEffect {
    /// Screen Space Ambient Occlusion.
    Ssao,
    /// Screen Space Global Illumination.
    Ssgi,
    /// Bloom glow effect.
    Bloom,
    /// ACES tone mapping.
    Aces,
    /// Vignette darkening.
    Vignette,
    /// Chromatic aberration distortion.
    ChromaticAberration,
    /// Color Look-Up Table grading.
    Lut,
}

impl fmt::Display for PassEffect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            PassEffect::Ssao => "SSAO",
            PassEffect::Ssgi => "SSGI",
            PassEffect::Bloom => "Bloom",
            PassEffect::Aces => "ACES",
            PassEffect::Vignette => "Vignette",
            PassEffect::ChromaticAberration => "ChromaticAberration",
            PassEffect::Lut => "LUT",
        };
        f.write_str(s)
    }
}

impl PassEffect {
    /// Returns the canonical stable name used for ordering, logging, and
    /// profiling.
    pub fn name(self) -> &'static str {
        match self {
            PassEffect::Ssao => "SSAO",
            PassEffect::Ssgi => "SSGI",
            PassEffect::Bloom => "Bloom",
            PassEffect::Aces => "ACES",
            PassEffect::Vignette => "Vignette",
            PassEffect::ChromaticAberration => "ChromaticAberration",
            PassEffect::Lut => "LUT",
        }
    }
}

/// Quality settings — the driver passes the current value to each pass.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum PassQuality {
    /// Effect is disabled.
    Off,
    /// Low quality (fastest, minimal samples).
    Low,
    /// Medium quality (balanced performance).
    Medium,
    /// High quality (more samples, better visuals).
    High,
    /// Ultra quality (maximum samples, best visuals).
    Ultra,
}

impl Default for PassQuality {
    fn default() -> Self {
        PassQuality::High
    }
}

impl PassQuality {
    /// Returns the numeric ordinal (Off=0, Low=1, Medium=2, High=3, Ultra=4).
    pub fn level(self) -> u8 {
        match self {
            PassQuality::Off => 0,
            PassQuality::Low => 1,
            PassQuality::Medium => 2,
            PassQuality::High => 3,
            PassQuality::Ultra => 4,
        }
    }
}

/// Per-camera context passed to each pass. Immutable from the pass's perspective
/// (the `PostStack` driver swaps the source/destination pair between passes).
#[derive(Debug, Clone)]
pub struct PostFxContext {
    /// Identifier of the source render target (engine-agnostic; the C# edge
    /// uses a `RenderTexture` instance).
    pub source: RenderTargetId,
    /// Identifier of the destination render target.
    pub destination: RenderTargetId,
    /// Camera being rendered (opaque handle, populated by the C# edge).
    pub camera: u64,
    /// Current quality setting.
    pub quality: PassQuality,
}

impl PostFxContext {
    /// Build a new context.
    pub fn new(
        source: RenderTargetId,
        destination: RenderTargetId,
        camera: u64,
        quality: PassQuality,
    ) -> Self {
        Self {
            source,
            destination,
            camera,
            quality,
        }
    }
}

/// Opaque render-target identifier; the C# edge resolves this to a
/// `RenderTexture` instance. 0 = None.
pub type RenderTargetId = u64;

/// Static description of a post-fx pass.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PassDescriptor {
    /// Stable effect name.
    pub effect: PassEffect,
    /// Shader name (e.g. `"Hidden/Phenotype/BloomPass"`).
    pub shader_name: String,
    /// Default enabled state.
    pub default_enabled: bool,
    /// Relative cost hint (0.0 = free, 1.0 = full-frame).
    pub cost: f32,
    /// Required shader keyword for the High variant.
    pub high_keyword: String,
    /// Description shown in inspectors.
    pub description: String,
}

/// Hexagonal port trait for a single post-fx pass.
pub trait PostFxPass: Send + Sync {
    /// Stable name used for ordering, logging, and profiling.
    fn name(&self) -> &str;

    /// Effect identifier.
    fn effect(&self) -> PassEffect;

    /// Relative cost hint (0.0 = free, 1.0 = full-frame).
    fn cost(&self) -> f32;

    /// Whether the pass should run on the current frame.
    fn is_enabled(&self) -> bool;

    /// Set the enabled state.
    fn set_enabled(&mut self, enabled: bool);

    /// Build pass-specific material and camera target allocations.
    fn on_setup(&mut self, ctx: &PostFxContext) -> PostFxResult<()>;

    /// Render the pass into the supplied source texture.
    fn on_render(&mut self, ctx: &PostFxContext) -> PostFxResult<()>;

    /// Free materials and temporary targets owned by this pass.
    fn on_dispose(&mut self);

    /// Validates that all required shader variants are present.
    fn validate_variants(
        &self,
        provider: &dyn PostFxShaderAvailability,
    ) -> Result<(), PostFxError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::postfx::ports::shader_availability::DefaultPostFxShaderAvailability;

    struct StubPass {
        effect: PassEffect,
        cost: f32,
        enabled: bool,
    }

    impl PostFxPass for StubPass {
        fn name(&self) -> &str {
            self.effect.name()
        }
        fn effect(&self) -> PassEffect {
            self.effect
        }
        fn cost(&self) -> f32 {
            self.cost
        }
        fn is_enabled(&self) -> bool {
            self.enabled
        }
        fn set_enabled(&mut self, e: bool) {
            self.enabled = e;
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
            _p: &dyn PostFxShaderAvailability,
        ) -> Result<(), PostFxError> {
            Ok(())
        }
    }

    #[test]
    fn pass_effect_name_is_stable() {
        assert_eq!(PassEffect::Ssao.name(), "SSAO");
        assert_eq!(PassEffect::Ssgi.name(), "SSGI");
        assert_eq!(PassEffect::Bloom.name(), "Bloom");
        assert_eq!(PassEffect::Aces.name(), "ACES");
        assert_eq!(PassEffect::Vignette.name(), "Vignette");
        assert_eq!(PassEffect::ChromaticAberration.name(), "ChromaticAberration");
        assert_eq!(PassEffect::Lut.name(), "LUT");
    }

    #[test]
    fn pass_quality_level() {
        assert_eq!(PassQuality::Off.level(), 0);
        assert_eq!(PassQuality::Low.level(), 1);
        assert_eq!(PassQuality::Medium.level(), 2);
        assert_eq!(PassQuality::High.level(), 3);
        assert_eq!(PassQuality::Ultra.level(), 4);
    }

    #[test]
    fn pass_descriptor_basic() {
        let d = PassDescriptor {
            effect: PassEffect::Bloom,
            shader_name: "Hidden/Phenotype/BrpBloom".into(),
            default_enabled: true,
            cost: 0.35,
            high_keyword: "BLOOM_HIGH".into(),
            description: "Bloom glow effect".into(),
        };
        assert_eq!(d.effect.name(), "Bloom");
        assert!((d.cost - 0.35).abs() < f32::EPSILON);
    }

    #[test]
    fn stub_pass_works() {
        let mut p = StubPass {
            effect: PassEffect::Bloom,
            cost: 0.1,
            enabled: true,
        };
        assert_eq!(p.name(), "Bloom");
        assert!(p.is_enabled());
        p.set_enabled(false);
        assert!(!p.is_enabled());
        let ctx = PostFxContext::new(1, 2, 42, PassQuality::High);
        assert!(p.on_setup(&ctx).is_ok());
        assert!(p.on_render(&ctx).is_ok());
        let provider = DefaultPostFxShaderAvailability;
        assert!(p.validate_variants(&provider).is_ok());
    }

    #[test]
    fn post_fx_context_constructs() {
        let ctx = PostFxContext::new(10, 20, 0, PassQuality::Medium);
        assert_eq!(ctx.source, 10);
        assert_eq!(ctx.destination, 20);
        assert_eq!(ctx.quality, PassQuality::Medium);
    }
}
