// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 KooshaPari <kooshapari@gmail.com>

//! Terrain shader types — vertex displacement, splatmap blending, and LOD
//! transitions for the terrain rendering pipeline.
//!
//! Each struct represents a named shader with embedded WGSL source and
//! bind/unbind lifecycle methods. The Rust core is engine-agnostic (ADR-004);
//! consumers pass the source string to the engine-side shader compilation
//! pipeline.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// TerrainHeightmapShader
// ---------------------------------------------------------------------------

/// Vertex-displacement shader that samples a heightmap texture and offsets
/// vertex positions along the surface normal.
///
/// Uniforms:
/// - `height_scale` — world-space multiplier for the heightmap sample.
/// - `height_offset` — constant world-space offset added after scaling.
/// - `heightmap_slot` — bind-group slot of the heightmap texture.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TerrainHeightmapShader {
    /// World-space scale factor applied to heightmap samples.
    pub height_scale: f32,
    /// Constant world-space offset added after scaling.
    pub height_offset: f32,
    /// Bind-group slot for the heightmap texture.
    pub heightmap_slot: u32,
}

impl Default for TerrainHeightmapShader {
    fn default() -> Self {
        Self {
            height_scale: 50.0,
            height_offset: 0.0,
            heightmap_slot: 0,
        }
    }
}

impl TerrainHeightmapShader {
    /// Create a new shader with default parameters.
    pub fn new() -> Self {
        Self::default()
    }

    /// Bind this shader to the current render pass.
    ///
    /// In a real engine this would set the active pipeline and bind groups;
    /// here it is a no-op documented hook.
    pub fn bind(&self) {
        // Engine-side: set pipeline, bind heightmap texture at `self.heightmap_slot`.
    }

    /// Unbind this shader from the current render pass.
    pub fn unbind(&self) {
        // Engine-side: restore previous pipeline state.
    }

    /// Return the WGSL source for this shader.
    pub fn source(&self) -> &'static str {
        TERRAIN_HEIGHTMAP_WGSL
    }
}

/// Embedded WGSL source for the terrain heightmap vertex-displacement shader.
const TERRAIN_HEIGHTMAP_WGSL: &str = r#"
struct Uniforms {
    height_scale: f32,
    height_offset: f32,
    heightmap_slot: u32,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var heightmap: texture_2d<f32>;
@group(0) @binding(2) var heightmap_sampler: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let height = textureSample(heightmap, heightmap_sampler, in.uv).r;
    let displacement = in.normal * (height * uniforms.height_scale + uniforms.height_offset);
    out.world_position = in.position + displacement;
    out.clip_position = vec4<f32>(out.world_position, 1.0);
    out.normal = in.normal;
    out.uv = in.uv;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.normal * 0.5 + 0.5, 1.0);
}
"#;

// ---------------------------------------------------------------------------
// TerrainSplatmapShader
// ---------------------------------------------------------------------------

/// Multi-texture blending shader that samples a splatmap (RGBA control map)
/// to blend up to four terrain texture layers (grass, rock, sand, snow).
///
/// Uniforms:
/// - `num_layers` — active texture layer count (1–4).
/// - `splatmap_slot` — bind-group slot of the splatmap texture.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TerrainSplatmapShader {
    /// Number of active texture layers (1–4).
    pub num_layers: u32,
    /// Bind-group slot for the splatmap texture.
    pub splatmap_slot: u32,
}

impl Default for TerrainSplatmapShader {
    fn default() -> Self {
        Self {
            num_layers: 4,
            splatmap_slot: 0,
        }
    }
}

impl TerrainSplatmapShader {
    /// Create a new shader with default parameters.
    pub fn new() -> Self {
        Self::default()
    }

    /// Bind this shader to the current render pass.
    pub fn bind(&self) {
        // Engine-side: set pipeline, bind splatmap + layer textures.
    }

    /// Unbind this shader from the current render pass.
    pub fn unbind(&self) {
        // Engine-side: restore previous pipeline state.
    }

    /// Return the WGSL source for this shader.
    pub fn source(&self) -> &'static str {
        TERRAIN_SPLATMAP_WGSL
    }
}

/// Embedded WGSL source for the terrain splatmap multi-texture blending shader.
const TERRAIN_SPLATMAP_WGSL: &str = r#"
struct Uniforms {
    num_layers: u32,
    splatmap_slot: u32,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var splatmap: texture_2d<f32>;
@group(0) @binding(2) var splatmap_sampler: sampler;
@group(0) @binding(3) var layer0: texture_2d<f32>;
@group(0) @binding(4) var layer1: texture_2d<f32>;
@group(0) @binding(5) var layer2: texture_2d<f32>;
@group(0) @binding(6) var layer3: texture_2d<f32>;
@group(0) @binding(7) var layer_sampler: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(in.position, 1.0);
    out.world_position = in.position;
    out.normal = in.normal;
    out.uv = in.uv;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let weights = textureSample(splatmap, splatmap_sampler, in.uv);
    var color = textureSample(layer0, layer_sampler, in.uv) * weights.r;
    if uniforms.num_layers > 1u {
        color += textureSample(layer1, layer_sampler, in.uv) * weights.g;
    }
    if uniforms.num_layers > 2u {
        color += textureSample(layer2, layer_sampler, in.uv) * weights.b;
    }
    if uniforms.num_layers > 3u {
        color += textureSample(layer3, layer_sampler, in.uv) * weights.a;
    }
    return color;
}
"#;

// ---------------------------------------------------------------------------
// TerrainLodTransitionShader
// ---------------------------------------------------------------------------

/// Smooth level-of-detail transition shader using dithered cross-fade between
/// two LOD tiers to avoid popping artifacts.
///
/// Uniforms:
/// - `fade_distance` — world-space distance over which the cross-fade occurs.
/// - `current_lod` — index of the current LOD tier being rendered.
/// - `target_lod` — index of the target LOD tier to blend toward.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TerrainLodTransitionShader {
    /// World-space distance over which the cross-fade occurs.
    pub fade_distance: f32,
    /// Index of the current LOD tier.
    pub current_lod: u32,
    /// Index of the target LOD tier to blend toward.
    pub target_lod: u32,
}

impl Default for TerrainLodTransitionShader {
    fn default() -> Self {
        Self {
            fade_distance: 30.0,
            current_lod: 0,
            target_lod: 1,
        }
    }
}

impl TerrainLodTransitionShader {
    /// Create a new shader with default parameters.
    pub fn new() -> Self {
        Self::default()
    }

    /// Bind this shader to the current render pass.
    pub fn bind(&self) {
        // Engine-side: set pipeline, bind both LOD mesh textures.
    }

    /// Unbind this shader from the current render pass.
    pub fn unbind(&self) {
        // Engine-side: restore previous pipeline state.
    }

    /// Return the WGSL source for this shader.
    pub fn source(&self) -> &'static str {
        TERRAIN_LOD_TRANSITION_WGSL
    }
}

/// Embedded WGSL source for the terrain LOD transition dithered cross-fade shader.
const TERRAIN_LOD_TRANSITION_WGSL: &str = r#"
struct Uniforms {
    fade_distance: f32,
    current_lod: u32,
    target_lod: u32,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var current_lod_tex: texture_2d<f32>;
@group(0) @binding(2) var target_lod_tex: texture_2d<f32>;
@group(0) @binding(3) var lod_sampler: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

// Bayer 2x2 dither matrix for cross-fade noise.
fn bayer2x2(p: vec2<f32>) -> f32 {
    let x = i32(p.x) % 2;
    let y = i32(p.y) % 2;
    let m = array<f32, 4>(0.0, 0.5, 0.75, 0.25);
    return m[y * 2 + x];
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(in.position, 1.0);
    out.world_position = in.position;
    out.normal = in.normal;
    out.uv = in.uv;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let current = textureSample(current_lod_tex, lod_sampler, in.uv);
    let target = textureSample(target_lod_tex, lod_sampler, in.uv);

    // Dithered cross-fade: threshold noise prevents hard transition line.
    let threshold = bayer2x2(in.clip_position.xy);
    let fade = clamp(length(in.world_position) / uniforms.fade_distance, 0.0, 1.0);
    if fade > threshold {
        return target;
    }
    return current;
}
"#;

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heightmap_shader_defaults() {
        let s = TerrainHeightmapShader::new();
        assert_eq!(s.height_scale, 50.0);
        assert_eq!(s.height_offset, 0.0);
        assert_eq!(s.heightmap_slot, 0);
    }

    #[test]
    fn heightmap_shader_source_non_empty() {
        let s = TerrainHeightmapShader::new();
        assert!(!s.source().is_empty());
        assert!(s.source().contains("vs_main"));
        assert!(s.source().contains("fs_main"));
    }

    #[test]
    fn splatmap_shader_defaults() {
        let s = TerrainSplatmapShader::new();
        assert_eq!(s.num_layers, 4);
        assert_eq!(s.splatmap_slot, 0);
    }

    #[test]
    fn splatmap_shader_source_non_empty() {
        let s = TerrainSplatmapShader::new();
        assert!(!s.source().is_empty());
        assert!(s.source().contains("splatmap"));
    }

    #[test]
    fn lod_transition_shader_defaults() {
        let s = TerrainLodTransitionShader::new();
        assert_eq!(s.fade_distance, 30.0);
        assert_eq!(s.current_lod, 0);
        assert_eq!(s.target_lod, 1);
    }

    #[test]
    fn lod_transition_shader_source_non_empty() {
        let s = TerrainLodTransitionShader::new();
        assert!(!s.source().is_empty());
        assert!(s.source().contains("bayer2x2"));
    }

    #[test]
    fn bind_unbind_do_not_panic() {
        let h = TerrainHeightmapShader::new();
        h.bind();
        h.unbind();

        let sp = TerrainSplatmapShader::new();
        sp.bind();
        sp.unbind();

        let lod = TerrainLodTransitionShader::new();
        lod.bind();
        lod.unbind();
    }
}
