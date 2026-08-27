// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 KooshaPari <kooshapari@gmail.com>

//! Integration tests for the compute shader framework.
//!
//! Tests verify that compute shader structs can be constructed, their WGSL
//! source is valid (non-empty, contains expected entry points), and that
//! dispatch configuration math is correct.

use phenotype_gfx::compute::mesh_generator::{MeshComputeShader, MeshGenConfig};
use phenotype_gfx::compute::sorting::{SortingComputeShader, SortingConfig};
use phenotype_gfx::compute::voxel_processor::{Operation, VoxelComputeShader};
use phenotype_gfx::compute::{ComputePipeline, ComputeShader, DispatchConfig, WorkgroupSize};

// ---------------------------------------------------------------------------
// Voxel compute tests
// ---------------------------------------------------------------------------

#[test]
fn test_voxel_compute_fill() {
    let shader = VoxelComputeShader::fill([32, 32, 32], 7);
    assert_eq!(shader.config.operation, Operation::Fill);
    assert_eq!(shader.config.material_id, 7);
    assert_eq!(shader.config.voxel_count(), 32 * 32 * 32);

    // WGSL source must contain the fill kernel.
    let src = shader.source();
    assert!(
        src.contains("fn fill("),
        "WGSL must contain fill entry point"
    );

    // Dispatch should cover all voxels with 64-thread workgroups.
    let cfg = DispatchConfig::one_d(shader.config.voxel_count(), 64);
    let (x, y, z) = cfg.dispatch_count();
    assert_eq!(y, 1);
    assert_eq!(z, 1);
    assert!(x > 0);
    assert_eq!(x * 64, 32 * 32 * 32); // exact multiple
}

#[test]
fn test_voxel_compute_carve() {
    let shader = VoxelComputeShader::carve([16, 16, 16], 3);
    assert_eq!(shader.config.operation, Operation::Carve);
    assert_eq!(shader.config.material_id, 3);

    let src = shader.source();
    assert!(
        src.contains("fn carve("),
        "WGSL must contain carve entry point"
    );
}

#[test]
fn test_voxel_compute_smooth() {
    let shader = VoxelComputeShader::smooth([8, 8, 8], 0.5);
    assert_eq!(shader.config.operation, Operation::Smooth);
    assert_eq!(shader.config.smooth_factor, 0.5);
    assert_eq!(shader.config.voxel_count(), 512);

    let src = shader.source();
    assert!(
        src.contains("fn smooth("),
        "WGSL must contain smooth entry point"
    );
}

// ---------------------------------------------------------------------------
// Mesh compute tests
// ---------------------------------------------------------------------------

#[test]
fn test_mesh_compute_generate() {
    let shader = MeshComputeShader::for_grid([16, 16, 16]);
    assert_eq!(shader.config.grid_size, [16, 16, 16]);
    assert_eq!(shader.config.voxel_count(), 4096);

    let src = shader.source();
    assert!(
        src.contains("fn generate("),
        "WGSL must contain generate entry point"
    );
    assert!(
        src.contains("fn emit_face("),
        "WGSL must contain emit_face helper"
    );

    // Worst-case: every voxel exposed on all 6 faces, 4 verts + 6 indices each.
    let cfg = MeshGenConfig {
        grid_size: [2, 2, 2],
        ..Default::default()
    };
    assert_eq!(cfg.voxel_count(), 8);
    assert_eq!(cfg.worst_case_vertices(), 8 * 6 * 4);
    assert_eq!(cfg.worst_case_indices(), 8 * 6 * 6);
}

#[test]
fn test_mesh_compute_dispatch() {
    let shader = MeshComputeShader::for_grid([4, 4, 4]);
    let cfg = DispatchConfig::one_d(shader.config.voxel_count(), 64);
    // 64 voxels / 64 threads = 1 workgroup
    assert_eq!(cfg.dispatch_count(), (1, 1, 1));

    let shader2 = MeshComputeShader::for_grid([16, 16, 16]);
    let cfg2 = DispatchConfig::one_d(shader2.config.voxel_count(), 64);
    // 4096 / 64 = 64 workgroups
    assert_eq!(cfg2.dispatch_count(), (64, 1, 1));
}

// ---------------------------------------------------------------------------
// Sorting compute tests
// ---------------------------------------------------------------------------

#[test]
fn test_sorting_compute_radix() {
    let shader = SortingComputeShader::for_count(4096);
    assert_eq!(shader.config.element_count, 4096);
    assert_eq!(shader.config.bits_per_digit, 4);
    assert_eq!(shader.config.key_bits, 32);

    let src = shader.source();
    assert!(
        src.contains("fn histogram_pass("),
        "WGSL must contain histogram_pass"
    );
    assert!(
        src.contains("fn prefix_sum_pass("),
        "WGSL must contain prefix_sum_pass"
    );
    assert!(
        src.contains("fn scatter_pass("),
        "WGSL must contain scatter_pass"
    );
}

#[test]
fn test_sorting_config_math() {
    let cfg = SortingConfig::default();
    // 32-bit key / 4-bit digits = 8 passes
    assert_eq!(cfg.digit_count(), 8);
    // 2^4 = 16 radix values
    assert_eq!(cfg.radix_size(), 16);
    assert_eq!(cfg.digit_mask(), 15);

    let workgroups = cfg.workgroup_count(256);
    assert_eq!(workgroups, 4); // ceil(1024 / 256)
    assert_eq!(cfg.histogram_size(256), 16 * 4); // 64
}

#[test]
fn test_sorting_compute_dispatch() {
    let _shader = SortingComputeShader::for_count(1000);
    let cfg = DispatchConfig {
        workgroup_size: WorkgroupSize::one_d(256),
        element_count: 1000,
        label: None,
    };
    // ceil(1000 / 256) = 4
    assert_eq!(cfg.dispatch_count(), (4, 1, 1));
}

// ---------------------------------------------------------------------------
// ComputePipeline tests
// ---------------------------------------------------------------------------

#[test]
fn test_compute_pipeline_assembly() {
    let voxel_cfg = DispatchConfig::one_d(4096, 64).with_label("voxel-fill");
    let pipeline = ComputePipeline::new("voxel-fill-pipeline", voxel_cfg);
    assert_eq!(pipeline.name, "voxel-fill-pipeline");
    assert_eq!(pipeline.dispatch_count(), (64, 1, 1));

    let sort_cfg = DispatchConfig {
        workgroup_size: WorkgroupSize::one_d(256),
        element_count: 1024,
        label: None,
    };
    let sort_pipeline = ComputePipeline::new("radix-sort", sort_cfg);
    assert_eq!(sort_pipeline.dispatch_count(), (4, 1, 1));
}

// ---------------------------------------------------------------------------
// WorkgroupSize tests
// ---------------------------------------------------------------------------

#[test]
fn test_workgroup_size_3d() {
    let ws = WorkgroupSize::three_d(8, 8, 8);
    assert_eq!(ws.total_threads(), 512);
    assert_eq!(ws.x, 8);
    assert_eq!(ws.y, 8);
    assert_eq!(ws.z, 8);
}
