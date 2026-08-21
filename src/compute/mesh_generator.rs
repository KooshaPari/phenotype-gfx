// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 KooshaPari <kooshapari@gmail.com>

//! GPU-accelerated parallel mesh generation from voxel data.
//!
//! [`MeshComputeShader`] generates triangle vertices and indices from a voxel
//! grid entirely on the GPU. The compute shader iterates over every voxel,
//! checks face adjacency with the six cardinal neighbours, and emits a quad
//! (two triangles) for each exposed face.
//!
//! ## Buffer layout
//!
//! - `@binding(0)` — uniforms (grid dimensions, vertex/index counts).
//! - `@binding(1)` — voxel data buffer (`array<u32>`, material IDs).
//! - `@binding(2)` — output vertex buffer (`array<vec4<f32>>`, xyz + material).
//! - `@binding(3)` — output index buffer (`array<u32>`).
//! - `@binding(4)` — atomic counter for vertex allocation.
//!
//! ## Vertex format
//!
//! Each emitted vertex is a `vec4<f32>`: `(x, y, z, material_id)`. Consumers
//! unpack this into their engine-specific vertex layout.

use serde::{Deserialize, Serialize};

use super::{ComputeShader, DispatchConfig};

/// Configuration for the mesh generation compute shader.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeshGenConfig {
    /// Grid dimensions along each axis.
    pub grid_size: [u32; 3],
    /// Maximum number of vertices the output buffer can hold.
    pub max_vertices: u32,
    /// Maximum number of indices the output buffer can hold.
    pub max_indices: u32,
}

impl Default for MeshGenConfig {
    fn default() -> Self {
        Self {
            grid_size: [16, 16, 16],
            max_vertices: 16 * 16 * 16 * 6 * 4, // worst case: 6 faces × 4 verts
            max_indices: 16 * 16 * 16 * 6 * 6,  // worst case: 6 faces × 6 indices
        }
    }
}

impl MeshGenConfig {
    /// Total number of voxels to iterate over.
    pub fn voxel_count(&self) -> u32 {
        self.grid_size[0] * self.grid_size[1] * self.grid_size[2]
    }

    /// Estimated worst-case vertex count.
    pub fn worst_case_vertices(&self) -> u32 {
        self.voxel_count() * 6 * 4
    }

    /// Estimated worst-case index count.
    pub fn worst_case_indices(&self) -> u32 {
        self.voxel_count() * 6 * 6
    }
}

/// GPU compute shader for parallel mesh generation from voxel grids.
///
/// Implements [`ComputeShader`] to provide embedded WGSL source for generating
/// triangle meshes on the GPU. Each voxel is processed independently; face
/// culling removes internal faces to produce an efficient mesh.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeshComputeShader {
    /// Configuration controlling grid size and buffer capacities.
    pub config: MeshGenConfig,
}

impl Default for MeshComputeShader {
    fn default() -> Self {
        Self {
            config: MeshGenConfig::default(),
        }
    }
}

impl MeshComputeShader {
    /// Create a new mesh compute shader with default configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a mesh compute shader for the given grid dimensions.
    pub fn for_grid(grid_size: [u32; 3]) -> Self {
        Self {
            config: MeshGenConfig {
                grid_size,
                ..Default::default()
            },
        }
    }

}

impl ComputeShader for MeshComputeShader {
    fn source(&self) -> &'static str {
        MESH_GENERATOR_WGSL
    }

    fn dispatch_config(&self) -> DispatchConfig {
        DispatchConfig::one_d(self.config.voxel_count(), 64)
    }

    fn bind(&self) {
        // Engine-side: set compute pipeline, bind voxel buffer + output buffers + counter.
    }

    fn unbind(&self) {
        // Engine-side: unbind compute pipeline, read back vertex/index counts.
    }
}

/// Embedded WGSL source for the parallel mesh generation compute shader.
///
/// Each workgroup processes 64 voxels. For each solid voxel, the shader checks
/// the six cardinal neighbours and emits a quad (4 vertices, 6 indices) for
/// each exposed face. An atomic counter ensures thread-safe vertex allocation.
const MESH_GENERATOR_WGSL: &str = r#"
struct MeshGenUniforms {
    grid_size_x: u32,
    grid_size_y: u32,
    grid_size_z: u32,
    max_vertices: u32,
    max_indices: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}

@group(0) @binding(0) var<uniform> uniforms: MeshGenUniforms;
@group(0) @binding(1) var<storage, read> voxels: array<u32>;
@group(0) @binding(2) var<storage, read_write> vertices: array<vec4<f32>>;
@group(0) @binding(3) var<storage, read_write> indices: array<u32>;
@group(0) @binding(4) var<storage, read_write> vertex_counter: atomic<u32>;
@group(0) @binding(5) var<storage, read_write> index_counter: atomic<u32>;

fn idx_to_coord(idx: u32) -> vec3<u32> {
    let z = idx / (uniforms.grid_size_x * uniforms.grid_size_y);
    let rem = idx % (uniforms.grid_size_x * uniforms.grid_size_y);
    let y = rem / uniforms.grid_size_x;
    let x = rem % uniforms.grid_size_x;
    return vec3<u32>(x, y, z);
}

fn coord_to_flat(x: i32, y: i32, z: i32) -> u32 {
    if x < 0 || y < 0 || z < 0 { return 4294967295u; }
    let ux = u32(x); let uy = u32(y); let uz = u32(z);
    if ux >= uniforms.grid_size_x || uy >= uniforms.grid_size_y || uz >= uniforms.grid_size_z {
        return 4294967295u;
    }
    return uz * uniforms.grid_size_x * uniforms.grid_size_y + uy * uniforms.grid_size_x + ux;
}

fn is_solid(flat: u32) -> bool {
    let total = uniforms.grid_size_x * uniforms.grid_size_y * uniforms.grid_size_z;
    return flat < total && voxels[flat] != 0u;
}

// Emit a face quad: 4 vertices + 6 indices.
fn emit_face(origin: vec3<f32>, face_normal: vec3<f32>, face_right: vec3<f32>,
             face_up: vec3<f32>, material: f32) {
    let base = atomicAdd(&vertex_counter, 4u);
    let idx_base = atomicAdd(&index_counter, 6u);

    if base + 4u > uniforms.max_vertices { return; }
    if idx_base + 6u > uniforms.max_indices { return; }

    let p0 = origin;
    let p1 = origin + face_right;
    let p2 = origin + face_right + face_up;
    let p3 = origin + face_up;

    vertices[base + 0u] = vec4<f32>(p0, material);
    vertices[base + 1u] = vec4<f32>(p1, material);
    vertices[base + 2u] = vec4<f32>(p2, material);
    vertices[base + 3u] = vec4<f32>(p3, material);

    indices[idx_base + 0u] = base + 0u;
    indices[idx_base + 1u] = base + 1u;
    indices[idx_base + 2u] = base + 2u;
    indices[idx_base + 3u] = base + 0u;
    indices[idx_base + 4u] = base + 2u;
    indices[idx_base + 5u] = base + 3u;
}

@compute @workgroup_size(64, 1, 1)
fn generate(@builtin(global_invocation_id) gid: vec3<u32>) {
    let total = uniforms.grid_size_x * uniforms.grid_size_y * uniforms.grid_size_z;
    let idx = gid.x;
    if idx >= total { return; }

    if voxels[idx] == 0u { return; }

    let coord = vec3<i32>(idx_to_coord(idx));
    let origin = vec3<f32>(coord);
    let material = f32(voxels[idx]);

    // +X face
    if !is_solid(coord_to_flat(coord.x + 1, coord.y, coord.z)) {
        emit_face(origin + vec3<f32>(1.0, 0.0, 0.0),
                  vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(0.0, 1.0, 0.0), vec3<f32>(0.0, 0.0, 1.0),
                  material);
    }
    // -X face
    if !is_solid(coord_to_flat(coord.x - 1, coord.y, coord.z)) {
        emit_face(origin,
                  vec3<f32>(-1.0, 0.0, 0.0), vec3<f32>(0.0, 0.0, 1.0), vec3<f32>(0.0, 1.0, 0.0),
                  material);
    }
    // +Y face
    if !is_solid(coord_to_flat(coord.x, coord.y + 1, coord.z)) {
        emit_face(origin + vec3<f32>(0.0, 1.0, 0.0),
                  vec3<f32>(0.0, 1.0, 0.0), vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(0.0, 0.0, 1.0),
                  material);
    }
    // -Y face
    if !is_solid(coord_to_flat(coord.x, coord.y - 1, coord.z)) {
        emit_face(origin,
                  vec3<f32>(0.0, -1.0, 0.0), vec3<f32>(0.0, 0.0, 1.0), vec3<f32>(1.0, 0.0, 0.0),
                  material);
    }
    // +Z face
    if !is_solid(coord_to_flat(coord.x, coord.y, coord.z + 1)) {
        emit_face(origin + vec3<f32>(0.0, 0.0, 1.0),
                  vec3<f32>(0.0, 0.0, 1.0), vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(0.0, 1.0, 0.0),
                  material);
    }
    // -Z face
    if !is_solid(coord_to_flat(coord.x, coord.y, coord.z - 1)) {
        emit_face(origin,
                  vec3<f32>(0.0, 0.0, -1.0), vec3<f32>(0.0, 1.0, 0.0), vec3<f32>(1.0, 0.0, 0.0),
                  material);
    }
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mesh_shader_defaults() {
        let s = MeshComputeShader::new();
        assert_eq!(s.config.grid_size, [16, 16, 16]);
    }

    #[test]
    fn mesh_shader_source_non_empty() {
        let s = MeshComputeShader::new();
        assert!(!s.source().is_empty());
        assert!(s.source().contains("@compute"));
        assert!(s.source().contains("generate"));
        assert!(s.source().contains("emit_face"));
    }

    #[test]
    fn mesh_shader_for_grid() {
        let s = MeshComputeShader::for_grid([32, 32, 32]);
        assert_eq!(s.config.grid_size, [32, 32, 32]);
        assert_eq!(s.config.voxel_count(), 32 * 32 * 32);
    }

    #[test]
    fn mesh_config_worst_case() {
        let cfg = MeshGenConfig {
            grid_size: [4, 4, 4],
            ..Default::default()
        };
        assert_eq!(cfg.voxel_count(), 64);
        // Each voxel can emit up to 6 faces, each face = 4 verts
        assert_eq!(cfg.worst_case_vertices(), 64 * 6 * 4);
        assert_eq!(cfg.worst_case_indices(), 64 * 6 * 6);
    }

    #[test]
    fn bind_unbind_do_not_panic() {
        let s = MeshComputeShader::new();
        s.bind();
        s.unbind();
    }

    #[test]
    fn dispatch_count_for_mesh_shader() {
        let s = MeshComputeShader::for_grid([8, 8, 8]);
        let dispatch = s.dispatch_config();
        assert_eq!(dispatch.element_count, 512);
        // ceil(512 / 64) = 8
        assert_eq!(dispatch.dispatch_count(), (8, 1, 1));
    }
}
