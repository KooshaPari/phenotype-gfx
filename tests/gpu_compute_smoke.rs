// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 KooshaPari <kooshapari@gmail.com>

//! Smoke test for the GPU compute pipeline (`compute::gpu_mesher`).
//!
//! What this test verifies:
//!
//! 1. `GpuMesher::new` returns either `Some(mesher)` (real adapter available)
//!    or `Ok(None)` (no adapter — common on headless CI). Either path is a
//!    pass; the test branches accordingly.
//! 2. When a mesher is available, the WGSL kernel compiles, a small voxel
//!    chunk round-trips through the GPU, and the output buffer shape matches
//!    the expected worst-case vertex/index capacity.
//! 3. The `MeshGenConfig` defaults round-trip through the GPU: voxel-count
//!    sent = voxel-count dispatched, workgroup count = `ceil(N / 64)`.
//!
//! No mock backend is required — wgpu 22's `force_fallback_adapter: true`
//! asks the OS for its software adapter (lavapipe / WARP / SwiftShader),
//! which is enough to exercise the full dispatch + readback path on
//! headless runners. When even that isn't available we still want the
//! compile + size-check assertions to run.

#![cfg(feature = "gpu")]

use phenotype_gfx::compute::gpu_mesher::{GpuMesher, VoxelChunkUpload};
use phenotype_gfx::compute::mesh_generator::{MeshGenConfig, MeshComputeShader};
use phenotype_gfx::compute::ComputeShader;

/// Helper: build a 16³ voxel chunk where every cell is solid material `1`.
fn solid_chunk() -> (Vec<u32>, [u32; 3]) {
    let grid = [16u32, 16, 16];
    let voxels = vec![1u32; (grid[0] * grid[1] * grid[2]) as usize];
    (voxels, grid)
}

#[test]
fn wgsl_source_contains_compute_entry_point() {
    let shader = MeshComputeShader::new();
    let src = shader.source();
    assert!(src.contains("@compute"), "WGSL must contain @compute attribute");
    assert!(
        src.contains("fn generate("),
        "WGSL must contain generate entry point"
    );
}

#[test]
fn mesh_config_worst_case_geometry() {
    // For a 16³ chunk, the worst case is every voxel fully exposed on all
    // six faces -> 16³ * 6 faces * 4 vertices = 98 304 vertices, 16³ * 6 * 6
    // = 24 576 indices. Guard the shader capacity expectations.
    let cfg = MeshGenConfig::default();
    assert_eq!(cfg.grid_size, [16, 16, 16]);
    assert_eq!(cfg.worst_case_vertices(), 16 * 16 * 16 * 6 * 4);
    assert_eq!(cfg.worst_case_indices(), 16 * 16 * 16 * 6 * 6);
}

#[test]
fn dispatch_count_matches_one_d_ceil_div() {
    let shader = MeshComputeShader::for_grid([16, 16, 16]);
    let dispatch = shader.dispatch_config();
    let (gx, gy, gz) = dispatch.dispatch_count();
    assert_eq!((gx, gy, gz), (64, 1, 1)); // ceil(4096 / 64)
    assert_eq!(dispatch.element_count, 4096);
}

#[test]
fn gpu_mesher_construction_is_graceful() {
    match GpuMesher::new(None) {
        Ok(Some(_mesher)) => {
            // Adapter found — try the actual dispatch below.
        }
        Ok(None) => {
            // No adapter at all (headless CI). Construction itself
            // succeeded with a graceful "no GPU" response — that's the
            // documented behaviour and a valid pass.
        }
        Err(e) => panic!("GpuMesher::new returned a hard error: {e}"),
    }
}

#[test]
fn gpu_mesher_dispatch_round_trip() {
    // Skip gracefully when no adapter exists. The construction test above
    // already covers the no-adapter path.
    let Ok(Some(mut mesher)) = GpuMesher::new(None) else {
        eprintln!("[gpu_compute_smoke] no adapter available — skipping dispatch");
        return;
    };

    let (voxels, grid) = solid_chunk();
    let output = mesher
        .dispatch(&VoxelChunkUpload {
            grid_size: grid,
            voxels: &voxels,
        })
        .expect("dispatch should succeed when adapter is available");

    // Workgroup count must be ceil(voxel_count / 64) along X.
    let expected_wg = mesher.config().voxel_count().div_ceil(64);
    assert_eq!(output.workgroups, (expected_wg, 1, 1));

    // A fully-solid 16³ chunk emits 6 faces per interior voxel + fewer on
    // boundary voxels (no neighbour). The exact number is determined by the
    // shader's face-emission rules; what we care about is:
    //   * it fits inside the worst-case buffer capacity,
    //   * each emitted vertex is a vec4<f32> = 16 bytes,
    //   * each emitted index is a u32 = 4 bytes.
    let cfg = mesher.config();
    assert!(
        output.vertex_count <= cfg.worst_case_vertices(),
        "vertex_count {} > worst_case_vertices {}",
        output.vertex_count,
        cfg.worst_case_vertices()
    );
    assert!(
        output.index_count <= cfg.worst_case_indices(),
        "index_count {} > worst_case_indices {}",
        output.index_count,
        cfg.worst_case_indices()
    );
    // Indices are emitted in groups of 6 (two triangles per quad).
    assert_eq!(output.index_count % 6, 0, "indices must be a multiple of 6");
    // Vertices are emitted in groups of 4 (one quad).
    assert_eq!(output.vertex_count % 4, 0, "vertices must be a multiple of 4");
    // For a solid 16³ block, every voxel has at least one exposed face, so
    // the counter should be well above zero.
    assert!(output.vertex_count > 0, "fully-solid chunk must emit vertices");
    assert!(output.index_count > 0, "fully-solid chunk must emit indices");
}

#[test]
fn gpu_mesher_rejects_wrong_voxel_count() {
    let Ok(Some(mut mesher)) = GpuMesher::new(None) else {
        return;
    };

    // Buffer is sized for 16³ = 4096 voxels; pass only 7.
    let wrong = vec![0u32; 7];
    let result = mesher.dispatch(&VoxelChunkUpload {
        grid_size: [16, 16, 16],
        voxels: &wrong,
    });
    assert!(
        matches!(
            result,
            Err(phenotype_gfx::compute::gpu_mesher::GpuError::VoxelCountMismatch { .. })
        ),
        "expected VoxelCountMismatch, got {result:?}"
    );
}

#[test]
fn gpu_mesher_empty_chunk_emits_zero_vertices() {
    let Ok(Some(mut mesher)) = GpuMesher::new(None) else {
        return;
    };

    let mut voxels = vec![0u32; 16 * 16 * 16]; // all air
    // Set a single voxel solid so we know the shader is *running* — pure-air
    // would still emit zero but the test would be ambiguous if the dispatch
    // itself failed silently.
    voxels[0] = 1;
    let output = mesher
        .dispatch(&VoxelChunkUpload {
            grid_size: [16, 16, 16],
            voxels: &voxels,
        })
        .expect("dispatch should succeed");

    // One isolated solid voxel on the -X/-Y/-Z corner: all 6 faces are exposed
    // (the WGSL `is_solid()` helper treats OOB neighbours as air). Each face
    // = 4 vertices / 6 indices.
    assert_eq!(output.vertex_count, 6 * 4);
    assert_eq!(output.index_count, 6 * 6);
}