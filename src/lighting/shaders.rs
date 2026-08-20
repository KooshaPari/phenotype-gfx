// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 KooshaPari <kooshapari@gmail.com>

//! Lighting shader types — ambient occlusion, directional light, point light,
//! and cascaded shadow mapping for the lighting pipeline.
//!
//! Each struct represents a named shader with embedded WGSL source and
//! bind/unbind lifecycle methods. The Rust core is engine-agnostic (ADR-004);
//! consumers pass the source string to the engine-side shader compilation
//! pipeline.

use glam::Vec3;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// AmbientOcclusionShader (SSAO)
// ---------------------------------------------------------------------------

/// Screen-space ambient occlusion shader using hemisphere kernel sampling.
///
/// Uniforms:
/// - `radius` — world-space sampling radius.
/// - `intensity` — multiplier for the final occlusion darkening.
/// - `bias` — depth bias to suppress self-occlusion.
/// - `kernel_size` — number of hemisphere samples per pixel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AmbientOcclusionShader {
    /// World-space sampling radius.
    pub radius: f32,
    /// Intensity multiplier for occlusion darkening.
    pub intensity: f32,
    /// Depth bias to suppress self-occlusion artifacts.
    pub bias: f32,
    /// Number of hemisphere samples per pixel.
    pub kernel_size: u32,
}

impl Default for AmbientOcclusionShader {
    fn default() -> Self {
        Self {
            radius: 0.5,
            intensity: 1.0,
            bias: 0.025,
            kernel_size: 16,
        }
    }
}

impl AmbientOcclusionShader {
    /// Create a new shader with default parameters.
    pub fn new() -> Self {
        Self::default()
    }

    /// Bind this shader to the current render pass.
    pub fn bind(&self) {
        // Engine-side: set pipeline, bind depth/normal textures.
    }

    /// Unbind this shader from the current render pass.
    pub fn unbind(&self) {
        // Engine-side: restore previous pipeline state.
    }

    /// Return the WGSL source for this shader.
    pub fn source(&self) -> &'static str {
        SSAO_WGSL
    }
}

/// Embedded WGSL source for the screen-space ambient occlusion shader.
const SSAO_WGSL: &str = r#"
struct Uniforms {
    radius: f32,
    intensity: f32,
    bias: f32,
    kernel_size: u32,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var depth_tex: texture_depth_2d;
@group(0) @binding(2) var normal_tex: texture_2d<f32>;
@group(0) @binding(3) var screen_sampler: sampler;
@group(0) @binding(4) var<storage, read> kernel: array<vec3<f32>>;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    // Full-screen triangle.
    let x = f32(i32(vertex_index) / 2) * 4.0 - 1.0;
    let y = f32(i32(vertex_index) % 2) * 4.0 - 1.0;
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) f32 {
    let depth = textureSample(depth_tex, screen_sampler, in.uv);
    let normal = textureSample(normal_tex, screen_sampler, in.uv).xyz;

    var occlusion = 0.0;
    let sample_count = min(uniforms.kernel_size, arrayLength(&kernel));
    for (var i = 0u; i < sample_count; i++) {
        let sample_offset = kernel[i] * uniforms.radius;
        let sample_uv = in.uv + sample_offset.xy;
        let sample_depth = textureSample(depth_tex, screen_sampler, sample_uv);
        let range_check = smoothstep(0.0, 1.0, uniforms.radius / abs(depth - sample_depth));
        occlusion += step(sample_depth, depth + uniforms.bias) * range_check;
    }
    occlusion = 1.0 - (occlusion / f32(sample_count)) * uniforms.intensity;
    return occlusion;
}
"#;

// ---------------------------------------------------------------------------
// DirectionalLightShader
// ---------------------------------------------------------------------------

/// Directional light shader for sun/moon rendering with Lambertian diffuse
/// and Blinn-Phong specular.
///
/// Uniforms:
/// - `direction` — normalized light direction vector.
/// - `color` — RGB light color.
/// - `intensity` — brightness multiplier.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DirectionalLightShader {
    /// Normalized light direction vector (points *toward* the light).
    pub direction: Vec3,
    /// RGB light color.
    pub color: [f32; 3],
    /// Brightness multiplier.
    pub intensity: f32,
}

impl Default for DirectionalLightShader {
    fn default() -> Self {
        Self {
            direction: Vec3::new(-0.5, -1.0, -0.3).normalize_or_zero(),
            color: [1.0, 0.95, 0.9],
            intensity: 1.0,
        }
    }
}

impl DirectionalLightShader {
    /// Create a new shader with default parameters.
    pub fn new() -> Self {
        Self::default()
    }

    /// Bind this shader to the current render pass.
    pub fn bind(&self) {
        // Engine-side: set pipeline, upload direction/color/intensity uniforms.
    }

    /// Unbind this shader from the current render pass.
    pub fn unbind(&self) {
        // Engine-side: restore previous pipeline state.
    }

    /// Return the WGSL source for this shader.
    pub fn source(&self) -> &'static str {
        DIRECTIONAL_LIGHT_WGSL
    }
}

/// Embedded WGSL source for the directional (sun/moon) light shader.
const DIRECTIONAL_LIGHT_WGSL: &str = r#"
struct Uniforms {
    direction: vec3<f32>,
    intensity: f32,
    color: vec3<f32>,
    _pad: f32,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var albedo_tex: texture_2d<f32>;
@group(0) @binding(2) var normal_tex: texture_2d<f32>;
@group(0) @binding(3) var material_tex: texture_2d<f32>;
@group(0) @binding(4) var light_sampler: sampler;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

@vertex
fn vs_main(in: VertexOutput) -> VertexOutput {
    return in;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let albedo = textureSample(albedo_tex, light_sampler, in.uv).rgb;
    let normal = normalize(in.normal);
    let light_dir = normalize(-uniforms.direction);

    // Lambertian diffuse.
    let n_dot_l = max(dot(normal, light_dir), 0.0);
    let diffuse = albedo * uniforms.color * n_dot_l * uniforms.intensity;

    // Simple ambient floor.
    let ambient = albedo * 0.1;

    return vec4<f32>(ambient + diffuse, 1.0);
}
"#;

// ---------------------------------------------------------------------------
// PointLightShader
// ---------------------------------------------------------------------------

/// Point light shader for localised light sources (torches, fires, lanterns)
/// with distance attenuation and Lambertian diffuse.
///
/// Uniforms:
/// - `position` — world-space position of the light.
/// - `color` — RGB light color.
/// - `intensity` — brightness multiplier.
/// - `attenuation` — quadratic attenuation factor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PointLightShader {
    /// World-space position of the light.
    pub position: Vec3,
    /// RGB light color.
    pub color: [f32; 3],
    /// Brightness multiplier.
    pub intensity: f32,
    /// Quadratic attenuation factor (higher = faster falloff).
    pub attenuation: f32,
}

impl Default for PointLightShader {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            color: [1.0, 0.8, 0.4],
            intensity: 1.0,
            attenuation: 1.0,
        }
    }
}

impl PointLightShader {
    /// Create a new shader with default parameters.
    pub fn new() -> Self {
        Self::default()
    }

    /// Bind this shader to the current render pass.
    pub fn bind(&self) {
        // Engine-side: set pipeline, upload position/color/intensity/attenuation uniforms.
    }

    /// Unbind this shader from the current render pass.
    pub fn unbind(&self) {
        // Engine-side: restore previous pipeline state.
    }

    /// Return the WGSL source for this shader.
    pub fn source(&self) -> &'static str {
        POINT_LIGHT_WGSL
    }
}

/// Embedded WGSL source for the point light shader with distance attenuation.
const POINT_LIGHT_WGSL: &str = r#"
struct Uniforms {
    position: vec3<f32>,
    intensity: f32,
    color: vec3<f32>,
    attenuation: f32,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var albedo_tex: texture_2d<f32>;
@group(0) @binding(2) var normal_tex: texture_2d<f32>;
@group(0) @binding(3) var point_sampler: sampler;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

@vertex
fn vs_main(in: VertexOutput) -> VertexOutput {
    return in;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let albedo = textureSample(albedo_tex, point_sampler, in.uv).rgb;
    let normal = normalize(in.normal);
    let light_vec = uniforms.position - in.world_position;
    let distance = length(light_vec);
    let light_dir = normalize(light_vec);

    // Attenuation: 1 / (1 + att * d^2).
    let atten = 1.0 / (1.0 + uniforms.attenuation * distance * distance);

    // Lambertian diffuse.
    let n_dot_l = max(dot(normal, light_dir), 0.0);
    let diffuse = albedo * uniforms.color * n_dot_l * uniforms.intensity * atten;

    // Ambient floor.
    let ambient = albedo * 0.05;

    return vec4<f32>(ambient + diffuse, 1.0);
}
"#;

// ---------------------------------------------------------------------------
// ShadowMappingShader
// ---------------------------------------------------------------------------

/// Cascaded shadow map shader for directional lights with configurable cascade
/// splits, bias, and PCF sampling.
///
/// Uniforms:
/// - `cascade_count` — number of active shadow cascades (1–4).
/// - `shadow_map_size` — resolution of each cascade shadow map (width = height).
/// - `bias` — depth bias per-sample to reduce shadow acne.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShadowMappingShader {
    /// Number of active shadow cascades (1–4).
    pub cascade_count: u32,
    /// Resolution of each cascade shadow map.
    pub shadow_map_size: u32,
    /// Depth bias to reduce shadow acne.
    pub bias: f32,
}

impl Default for ShadowMappingShader {
    fn default() -> Self {
        Self {
            cascade_count: 4,
            shadow_map_size: 2048,
            bias: 0.005,
        }
    }
}

impl ShadowMappingShader {
    /// Create a new shader with default parameters.
    pub fn new() -> Self {
        Self::default()
    }

    /// Bind this shader to the current render pass.
    pub fn bind(&self) {
        // Engine-side: set pipeline, bind cascade shadow map array + light-space matrices.
    }

    /// Unbind this shader from the current render pass.
    pub fn unbind(&self) {
        // Engine-side: restore previous pipeline state.
    }

    /// Return the WGSL source for this shader.
    pub fn source(&self) -> &'static str {
        SHADOW_MAPPING_WGSL
    }
}

/// Embedded WGSL source for the cascaded shadow map shader.
const SHADOW_MAPPING_WGSL: &str = r#"
struct Uniforms {
    cascade_count: u32,
    shadow_map_size: u32,
    bias: f32,
    _pad: f32,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var shadow_maps: texture_depth_2d_array;
@group(0) @binding(2) var shadow_sampler: sampler_comparison;
@group(0) @binding(3) var<storage, read> cascade_splits: array<f32>;
@group(0) @binding(4) var<storage, read> light_space_matrices: array<mat4x4<f32>>;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

@vertex
fn vs_main(in: VertexOutput) -> VertexOutput {
    return in;
}

// PCF shadow sampling with a 3x3 kernel.
fn sample_shadow(shadow_pos: vec4<f32>, cascade_idx: u32) -> f32 {
    let proj_coords = shadow_pos.xyz / shadow_pos.w;
    let uv = proj_coords.xy * 0.5 + 0.5;
    let current_depth = proj_coords.z - uniforms.bias;
    let texel_size = 1.0 / f32(uniforms.shadow_map_size);
    var shadow = 0.0;

    for (var x = -1; x <= 1; x++) {
        for (var y = -1; y <= 1; y++) {
            let offset = vec2<f32>(f32(x), f32(y)) * texel_size;
            shadow += textureSampleCompareLevel(
                shadow_maps, shadow_sampler,
                uv + offset, cascade_idx, current_depth
            );
        }
    }
    return shadow / 9.0;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) f32 {
    let view_distance = length(in.world_position);

    // Select cascade based on view distance.
    var cascade_idx = 0u;
    let count = min(uniforms.cascade_count, arrayLength(&cascade_splits));
    for (var i = 0u; i < count; i++) {
        if view_distance > cascade_splits[i] {
            cascade_idx = i + 1u;
        }
    }
    cascade_idx = min(cascade_idx, count - 1u);

    let shadow_pos = light_space_matrices[cascade_idx] * vec4<f32>(in.world_position, 1.0);
    return sample_shadow(shadow_pos, cascade_idx);
}
"#;

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ao_shader_defaults() {
        let s = AmbientOcclusionShader::new();
        assert_eq!(s.radius, 0.5);
        assert_eq!(s.intensity, 1.0);
        assert_eq!(s.bias, 0.025);
        assert_eq!(s.kernel_size, 16);
    }

    #[test]
    fn ao_shader_source_non_empty() {
        let s = AmbientOcclusionShader::new();
        assert!(!s.source().is_empty());
        assert!(s.source().contains("occlusion"));
    }

    #[test]
    fn directional_light_shader_defaults() {
        let s = DirectionalLightShader::new();
        assert!((s.direction.length() - 1.0).abs() < 0.001);
        assert_eq!(s.intensity, 1.0);
    }

    #[test]
    fn directional_light_shader_source_non_empty() {
        let s = DirectionalLightShader::new();
        assert!(!s.source().is_empty());
        assert!(s.source().contains("Lambertian"));
    }

    #[test]
    fn point_light_shader_defaults() {
        let s = PointLightShader::new();
        assert_eq!(s.position, Vec3::ZERO);
        assert_eq!(s.attenuation, 1.0);
        assert_eq!(s.intensity, 1.0);
    }

    #[test]
    fn point_light_shader_source_non_empty() {
        let s = PointLightShader::new();
        assert!(!s.source().is_empty());
        assert!(s.source().contains("atten"));
    }

    #[test]
    fn shadow_mapping_shader_defaults() {
        let s = ShadowMappingShader::new();
        assert_eq!(s.cascade_count, 4);
        assert_eq!(s.shadow_map_size, 2048);
        assert_eq!(s.bias, 0.005);
    }

    #[test]
    fn shadow_mapping_shader_source_non_empty() {
        let s = ShadowMappingShader::new();
        assert!(!s.source().is_empty());
        assert!(s.source().contains("sample_shadow"));
    }

    #[test]
    fn bind_unbind_do_not_panic() {
        let ao = AmbientOcclusionShader::new();
        ao.bind();
        ao.unbind();

        let dir = DirectionalLightShader::new();
        dir.bind();
        dir.unbind();

        let pt = PointLightShader::new();
        pt.bind();
        pt.unbind();

        let shadow = ShadowMappingShader::new();
        shadow.bind();
        shadow.unbind();
    }
}
