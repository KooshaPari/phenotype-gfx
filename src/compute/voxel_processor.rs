// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 KooshaPari <kooshapari@gmail.com>

//! GPU-accelerated voxel bulk operations: fill, carve, and smooth.
//!
//! [`VoxelComputeShader`] provides WGSL compute shaders for processing voxel
//! data entirely on the GPU. Each operation (fill, carve, smooth) has its own
//! WGSL entry point; the Rust side exposes a unified [`ComputeShader`]
//! implementation with an [`Operation`] enum to select the desired kernel.
//!
//! ## Operations
//!
//! | Operation | Description |
//! |-----------|-------------|
//! | `Fill`    | Write a material ID to every voxel in a region. |
//! | `Carve`   | Remove voxels matching a condition (e.g. empty or specific material). |
//! | `Smooth`  | Apply a 3×3×3 averaging filter to smooth voxel surfaces. |
//!
//! ## Buffer layout
//!
//! The WGSL shaders expect the following storage bindings:
//!
//! - `@binding(0)` — uniforms (grid dimensions, operation params).
//! - `@binding(1)` — voxel data buffer (`array<u32>`, material IDs).
//! - `@binding(2)` — scratch buffer for double-buffered smooth passes.

use serde::{Deserialize, Serialize};

use super::{ComputeShader, DispatchConfig};

/// The type of bulk voxel operation to dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Operation {
    /// Write a material ID to every voxel in the target region.
    Fill,
    /// Remove voxels matching a condition.
    Carve,
    /// Apply a 3×3×3 averaging filter for surface smoothing.
    Smooth,
}

/// Configuration for a voxel processing compute shader.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoxelProcessConfig {
    /// Grid dimensions along each axis.
    pub grid_size: [u32; 3],
    /// The operation to perform.
    pub operation: Operation,
    /// Material ID used by `Fill` and `Carve` operations.
    pub material_id: u32,
    /// Smoothing factor (0.0 = no smoothing, 1.0 = full average) for `Smooth`.
    pub smooth_factor: f32,
}

impl Default for VoxelProcessConfig {
    fn default() -> Self {
        Self {
            grid_size: [16, 16, 16],
            operation: Operation::Fill,
            material_id: 1,
            smooth_factor: 0.5,
        }
    }
}

impl VoxelProcessConfig {
    /// Total number of voxels in the grid.
    pub fn voxel_count(&self) -> u32 {
        self.grid_size[0] * self.grid_size[1] * self.grid_size[2]
    }
}

/// GPU compute shader for bulk voxel operations (fill, carve, smooth).
///
/// Implements [`ComputeShader`] to provide embedded WGSL source and dispatch
/// configuration for GPU-accelerated voxel processing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(Default)]
pub struct VoxelComputeShader {
    /// Configuration controlling grid size, operation type, and parameters.
    pub config: VoxelProcessConfig,
}


impl VoxelComputeShader {
    /// Create a new voxel compute shader with default configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a shader configured for a fill operation.
    pub fn fill(grid_size: [u32; 3], material_id: u32) -> Self {
        Self {
            config: VoxelProcessConfig {
                grid_size,
                operation: Operation::Fill,
                material_id,
                smooth_factor: 0.0,
            },
        }
    }

    /// Create a shader configured for a carve operation.
    pub fn carve(grid_size: [u32; 3], material_id: u32) -> Self {
        Self {
            config: VoxelProcessConfig {
                grid_size,
                operation: Operation::Carve,
                material_id,
                smooth_factor: 0.0,
            },
        }
    }

    /// Create a shader configured for a smooth operation.
    pub fn smooth(grid_size: [u32; 3], smooth_factor: f32) -> Self {
        Self {
            config: VoxelProcessConfig {
                grid_size,
                operation: Operation::Smooth,
                material_id: 0,
                smooth_factor,
            },
        }
    }
}

impl ComputeShader for VoxelComputeShader {
    fn source(&self) -> &'static str {
        VOXEL_PROCESSOR_WGSL
    }

    fn dispatch_config(&self) -> DispatchConfig {
        DispatchConfig::one_d(self.config.voxel_count(), 64)
    }

    fn bind(&self) {
        // Engine-side: set compute pipeline, bind voxel storage buffer + uniforms.
    }

    fn unbind(&self) {
        // Engine-side: unbind compute pipeline, release storage bindings.
    }
}

/// Embedded WGSL source for voxel processing compute shaders.
///
/// This shader supports three operations selected at runtime via a uniform
/// `operation` field:
///
/// - **Fill (0)**: Writes `material_id` to every voxel.
/// - **Carve (1)**: Sets voxels matching `material_id` to 0 (empty).
/// - **Smooth (2)**: 3x3x3 neighbourhood averaging pass.
const VOXEL_PROCESSOR_WGSL: &str = r#"
struct VoxelizeUniforms {
    grid_size_x: u32,
    grid_size_y: u32,
    grid_size_z: u32,
    operation: u32,       // 0 = fill, 1 = carve, 2 = smooth
    material_id: u32,
    smooth_factor: f32,
    _pad0: u32,
    _pad1: u32,
}

@group(0) @binding(0) var<uniform> uniforms: VoxelizeUniforms;
@group(0) @binding(1) var<storage, read_write> voxels: array<u32>;
@group(0) @binding(2) var<storage, read_write> scratch: array<u32>;

// Flat index → 3-D coordinate.
fn idx_to_coord(idx: u32) -> vec3<u32> {
    let z = idx / (uniforms.grid_size_x * uniforms.grid_size_y);
    let rem = idx % (uniforms.grid_size_x * uniforms.grid_size_y);
    let y = rem / uniforms.grid_size_x;
    let x = rem % uniforms.grid_size_x;
    return vec3<u32>(x, y, z);
}

// 3-D coordinate → flat index; returns u32::MAX for out-of-bounds.
fn coord_to_idx(coord: vec3<i32>) -> u32 {
    if coord.x < 0 || coord.y < 0 || coord.z < 0 { return 4294967295u; }
    let ux = u32(coord.x);
    let uy = u32(coord.y);
    let uz = u32(coord.z);
    if ux >= uniforms.grid_size_x || uy >= uniforms.grid_size_y || uz >= uniforms.grid_size_z {
        return 4294967295u;
    }
    return uz * uniforms.grid_size_x * uniforms.grid_size_y
         + uy * uniforms.grid_size_x
         + ux;
}

// Fill: write material_id to every voxel.
@compute @workgroup_size(64, 1, 1)
fn fill(@builtin(global_invocation_id) gid: vec3<u32>) {
    let total = uniforms.grid_size_x * uniforms.grid_size_y * uniforms.grid_size_z;
    let idx = gid.x;
    if idx >= total { return; }
    voxels[idx] = uniforms.material_id;
}

// Carve: set voxels matching material_id to 0 (empty).
@compute @workgroup_size(64, 1, 1)
fn carve(@builtin(global_invocation_id) gid: vec3<u32>) {
    let total = uniforms.grid_size_x * uniforms.grid_size_y * uniforms.grid_size_z;
    let idx = gid.x;
    if idx >= total { return; }
    if voxels[idx] == uniforms.material_id {
        voxels[idx] = 0u;
    }
}

// Smooth: 3x3x3 neighbourhood averaging pass.
@compute @workgroup_size(64, 1, 1)
fn smooth(@builtin(global_invocation_id) gid: vec3<u32>) {
    let total = uniforms.grid_size_x * uniforms.grid_size_y * uniforms.grid_size_z;
    let idx = gid.x;
    if idx >= total { return; }

    // Copy source to scratch on first invocation (stub — real impl uses
    // double-buffering or barrier).
    scratch[idx] = voxels[idx];

    let coord = vec3<i32>(idx_to_coord(idx));
    var sum: f32 = 0.0;
    var count: f32 = 0.0;

    for (var dz = -1; dz <= 1; dz++) {
        for (var dy = -1; dy <= 1; dy++) {
            for (var dx = -1; dx <= 1; dx++) {
                let neighbour = coord_to_idx(coord + vec3<i32>(dx, dy, dz));
                if neighbour < total {
                    sum += f32(scratch[neighbour]);
                    count += 1.0;
                }
            }
        }
    }

    let avg = sum / count;
    let original = f32(scratch[idx]);
    voxels[idx] = u32(mix(original, avg, uniforms.smooth_factor));
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voxel_shader_defaults() {
        let s = VoxelComputeShader::new();
        assert_eq!(s.config.grid_size, [16, 16, 16]);
        assert_eq!(s.config.operation, Operation::Fill);
        assert_eq!(s.config.material_id, 1);
    }

    #[test]
    fn voxel_shader_source_non_empty() {
        let s = VoxelComputeShader::new();
        assert!(!s.source().is_empty());
        assert!(s.source().contains("@compute"));
        assert!(s.source().contains("workgroup_size(64"));
    }

    #[test]
    fn voxel_shader_fill_operation() {
        let s = VoxelComputeShader::fill([32, 32, 32], 5);
        assert_eq!(s.config.operation, Operation::Fill);
        assert_eq!(s.config.material_id, 5);
        assert_eq!(s.config.voxel_count(), 32 * 32 * 32);
    }

    #[test]
    fn voxel_shader_carve_operation() {
        let s = VoxelComputeShader::carve([8, 8, 8], 3);
        assert_eq!(s.config.operation, Operation::Carve);
        assert_eq!(s.config.material_id, 3);
    }

    #[test]
    fn voxel_shader_smooth_operation() {
        let s = VoxelComputeShader::smooth([16, 16, 16], 0.75);
        assert_eq!(s.config.operation, Operation::Smooth);
        assert_eq!(s.config.smooth_factor, 0.75);
    }

    #[test]
    fn voxel_config_voxel_count() {
        let cfg = VoxelProcessConfig {
            grid_size: [10, 20, 30],
            ..Default::default()
        };
        assert_eq!(cfg.voxel_count(), 6000);
    }

    #[test]
    fn bind_unbind_do_not_panic() {
        let s = VoxelComputeShader::new();
        s.bind();
        s.unbind();
    }

    #[test]
    fn dispatch_count_for_voxel_shader() {
        let s = VoxelComputeShader::fill([4, 4, 4], 1);
        let dispatch = s.dispatch_config();
        assert_eq!(dispatch.element_count, 64);
        assert_eq!(dispatch.dispatch_count(), (1, 1, 1));
    }
}
