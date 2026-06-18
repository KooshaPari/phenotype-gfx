// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 KooshaPari <kooshapari@gmail.com>

//! HLSL shader source — preserved verbatim from the C# upstream
//! (`phenotype-postfx/Runtime/Shaders/*.shader`).
//!
//! These are kept as `&str` constants for documentation and for the C# edge
//! (under `unity/postfx-shaders/`) which loads them as the source of truth.
//! They are not transpiled or executed by the Rust core; the Rust core only
//! describes pass shapes (see [`crate::postfx::post_stack::PostStack`]) and
//! validates availability via [`crate::postfx::ports::shader_availability`].
//!
//! ## Why raw HLSL
//!
//! Per ADR-004 (single Rust core + thin FFI edges), the engine-specific shader
//! format remains the responsibility of the engine edge. The Rust core is
//! engine-agnostic and exposes the pass shape as data.
//!
//! ## Mapping
//!
//! | Rust constant                | Upstream `.shader` file                          |
//! |------------------------------|--------------------------------------------------|
//! | [`BLOOM_PASS_SHADER`]        | `BloomPass.shader`                               |
//! | [`BRP_BLOOM_SHADER`]         | `BrpBloom.shader`                                |
//! | [`BRP_ACES_SHADER`]          | `BrpACES.shader`                                 |
//! | [`COLOR_GRADING_LUT_SHADER`] | `ColorGradingLUT.shader`                         |
//! | [`CHROMATIC_ABERRATION_SHADER`] | `ChromaticAberration.shader`                  |
//! | [`VIGNETTE_SHADER`]          | `Vignette.shader`                                |
//! | [`SCREEN_SPACE_AO_SHADER`]   | `ScreenSpaceAO.shader`                           |
//! | [`SCREEN_SPACE_GI_SHADER`]   | `ScreenSpaceGI.shader`                           |
//! | [`SSAO_PASS_SHADER`]         | `SSAOPass.shader`                                |

/// `Hidden/Phenotype/BloomPass` — multi-pass bloom (threshold → blur H → blur V → composite).
pub const BLOOM_PASS_SHADER: &str = include_str!("../../unity/postfx-shaders/BloomPass.shader");

/// `Hidden/Phenotype/BrpBloom` — single-pass bloom for the built-in render pipeline.
pub const BRP_BLOOM_SHADER: &str = include_str!("../../unity/postfx-shaders/BrpBloom.shader");

/// `Hidden/Phenotype/BrpACES` — BRP ACES tonemapping.
pub const BRP_ACES_SHADER: &str = include_str!("../../unity/postfx-shaders/BrpACES.shader");

/// `Hidden/Phenotype/ColorGradingLUT` — LUT color grading.
pub const COLOR_GRADING_LUT_SHADER: &str =
    include_str!("../../unity/postfx-shaders/ColorGradingLUT.shader");

/// `Hidden/Phenotype/ChromaticAberration` — RGB-channel offset.
pub const CHROMATIC_ABERRATION_SHADER: &str =
    include_str!("../../unity/postfx-shaders/ChromaticAberration.shader");

/// `Hidden/Phenotype/Vignette` — radial darkening at the edges.
pub const VIGNETTE_SHADER: &str = include_str!("../../unity/postfx-shaders/Vignette.shader");

/// `Hidden/Phenotype/ScreenSpaceAO` — SAO (HBAO-style) ambient occlusion.
pub const SCREEN_SPACE_AO_SHADER: &str =
    include_str!("../../unity/postfx-shaders/ScreenSpaceAO.shader");

/// `Hidden/Phenotype/ScreenSpaceGI` — screen-space global illumination.
pub const SCREEN_SPACE_GI_SHADER: &str =
    include_str!("../../unity/postfx-shaders/ScreenSpaceGI.shader");

/// `Hidden/Phenotype/SSAOPass` — main SSAO pass with kernel sampling.
pub const SSAO_PASS_SHADER: &str = include_str!("../../unity/postfx-shaders/SSAOPass.shader");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shader_constants_are_non_empty() {
        assert!(!BLOOM_PASS_SHADER.is_empty());
        assert!(!BRP_BLOOM_SHADER.is_empty());
        assert!(!BRP_ACES_SHADER.is_empty());
        assert!(!COLOR_GRADING_LUT_SHADER.is_empty());
        assert!(!CHROMATIC_ABERRATION_SHADER.is_empty());
        assert!(!VIGNETTE_SHADER.is_empty());
        assert!(!SCREEN_SPACE_AO_SHADER.is_empty());
        assert!(!SCREEN_SPACE_GI_SHADER.is_empty());
        assert!(!SSAO_PASS_SHADER.is_empty());
    }
}
