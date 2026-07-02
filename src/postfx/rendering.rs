// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 KooshaPari <kooshapari@gmail.com>

//! Engine-agnostic render-target + material handle types (used by the C# edge).
//!
//! Mirrors the C# `WaterMaterial` / `WaterRenderer` / `WaterShader` pattern from
//! `phenotype-water`: kept as `#[deprecated]` pass-throughs. The real logic now
//! lives in [`crate::postfx::post_stack`] and the individual pass files. The
//! C# edge (in `unity/postfx/`) consumes the Rust core via P/Invoke.

/// Stable material handle (just an integer id).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MaterialHandle {
    /// Opaque material id.
    pub id: u64,
}

impl MaterialHandle {
    /// New handle.
    pub fn new(id: u64) -> Self {
        Self { id }
    }
}

/// Render-target descriptor (color format + width/height).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenderTarget {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Color format (engine-specific string).
    pub color_format: String,
}

impl RenderTarget {
    /// New render target.
    pub fn new(width: u32, height: u32, color_format: impl Into<String>) -> Self {
        Self {
            width,
            height,
            color_format: color_format.into(),
        }
    }
}

/// `#[deprecated]` pass-through material type. Kept for C# edge compatibility.
/// The real per-pass material config is now in [`crate::postfx::bloom_pass::BloomConfig`]
/// and [`crate::postfx::ssao_pass::SsaoConfig`].
#[deprecated(
    since = "0.2.0",
    note = "Use the per-pass config types (BloomConfig, SsaoConfig, ...) directly; this type is a pass-through kept for C# edge compatibility."
)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PostFxMaterial {
    /// Material handle id.
    pub id: MaterialHandle,
    /// Shader name (e.g. `"Hidden/Phenotype/BloomPass"`).
    pub shader_name: String,
}

#[allow(deprecated)]
impl PostFxMaterial {
    /// New material.
    pub fn new(id: u64, shader_name: impl Into<String>) -> Self {
        Self {
            id: MaterialHandle::new(id),
            shader_name: shader_name.into(),
        }
    }
}

/// `#[deprecated]` pass-through shader name type.
#[deprecated(
    since = "0.2.0",
    note = "Use the engine-side shader lookup; this type is a name-only pass-through kept for C# edge compatibility."
)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PostFxShader {
    /// Shader name.
    pub name: String,
}

#[allow(deprecated)]
impl PostFxShader {
    /// New shader.
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

/// `#[deprecated]` orchestrator that holds a material + shader + render target.
/// The real orchestration is in [`crate::postfx::post_stack::PostStack`].
#[deprecated(
    since = "0.2.0",
    note = "Use PostStack::describe_passes() + per-pass configs directly; this orchestrator is a pass-through kept for C# edge compatibility."
)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PostFxRenderer {
    /// Material handle.
    pub material: MaterialHandle,
    /// Shader name.
    pub shader_name: String,
    /// Target render target.
    pub target: RenderTarget,
}

#[allow(deprecated)]
impl PostFxRenderer {
    /// New renderer.
    pub fn new(material_id: u64, shader_name: impl Into<String>, target: RenderTarget) -> Self {
        Self {
            material: MaterialHandle::new(material_id),
            shader_name: shader_name.into(),
            target,
        }
    }
}

use serde::{Deserialize, Serialize};

#[cfg(test)]
#[allow(deprecated)]
mod tests {
    use super::*;

    #[test]
    fn material_handle_works() {
        let h = MaterialHandle::new(42);
        assert_eq!(h.id, 42);
    }

    #[test]
    fn render_target_works() {
        let t = RenderTarget::new(1920, 1080, "R8G8B8A8_UNorm");
        assert_eq!(t.width, 1920);
        assert_eq!(t.height, 1080);
        assert_eq!(t.color_format, "R8G8B8A8_UNorm");
    }

    #[test]
    fn deprecated_types_round_trip() {
        let m = PostFxMaterial::new(1, "Hidden/Bloom");
        assert_eq!(m.shader_name, "Hidden/Bloom");
        let s = PostFxShader::new("Hidden/Bloom");
        assert_eq!(s.name, "Hidden/Bloom");
        let r = PostFxRenderer::new(1, "Hidden/Bloom", RenderTarget::new(1, 1, "X"));
        assert_eq!(r.material.id, 1);
    }
}
