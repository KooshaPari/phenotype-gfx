//! Cross-module integration test for the voxel → LOD → streaming pipeline.
//!
//! This test verifies that the voxel kernel, LOD selection, and streaming
//! eviction logic compose correctly to produce the expected render lifecycle.

use phenotype_gfx::lod::{plan_chunk_render, ChunkRenderPlan, LodRingPlan, RingRole};
use phenotype_gfx::streaming::{ring_distance, ChunkState, EvictionKey, WindowPolicy};
use phenotype_gfx::voxel::chunk::{Chunk, ChunkId, ChunkView, CHUNK_EDGE};
use phenotype_gfx::voxel::greedy_mesher::GreedyMesher;
use phenotype_gfx::voxel::lod::LodLevel;
use phenotype_gfx::voxel::material::MaterialId;
use phenotype_gfx::voxel::mesh::Mesher;
use phenotype_gfx::voxel::coord::ChunkCoord;

fn idx(x: i32, y: i32, z: i32) -> usize {
    x as usize + y as usize * CHUNK_EDGE + z as usize * CHUNK_EDGE * CHUNK_EDGE
}

fn coord(cx: i32, cy: i32, cz: i32) -> ChunkCoord {
    ChunkCoord { cx, cy, cz }
}

/// Test: Voxel creation → Greedy meshing → LOD selection → Streaming eviction
///
/// 1. Create a chunk with some voxels.
/// 2. Mesh it using the GreedyMesher.
/// 3. Select LOD based on distance.
/// 4. Verify the chunk is correctly classified by the streaming window.
#[test]
fn voxel_to_streaming_pipeline() {
    // 1. Create a chunk with some voxels.
    let mut chunk = Chunk::<MaterialId>::default();
    chunk.voxels[idx(0, 0, 0)] = MaterialId(1);
    chunk.voxels[idx(1, 1, 1)] = MaterialId(2);
    chunk.voxels[idx(2, 2, 2)] = MaterialId(3);

    let view = ChunkView {
        id: ChunkId(0),
        voxels: &chunk.voxels,
    };

    // 2. Mesh it using the GreedyMesher.
    let mesh = GreedyMesher::<MaterialId>::mesh_greedy(view, LodLevel(0))
        .expect("greedy mesh should succeed");
    assert!(!mesh.vertices.is_empty(), "mesh should not be empty");

    // 3. Select LOD based on distance.
    let anchor = coord(0, 0, 0);
    let chunk_coord = coord(0, 0, 0); // Same chunk as anchor
    let policy = WindowPolicy::default();

    // Verify ring distance is 0 for the same chunk.
    assert_eq!(ring_distance(chunk_coord, anchor, policy.vy_weight), 0);

    // 4. Verify the chunk is correctly classified by the streaming window.
    // A chunk at ring 0 should be Meshed.
    let state = policy.classify(chunk_coord, anchor);
    assert_eq!(state, ChunkState::Meshed, "chunk at ring 0 should be Meshed");
    assert!(state.has_mesh(), "Meshed state should have mesh");
    assert!(state.is_resident(), "Meshed state should be resident");
}

/// Test: Streaming eviction ordering for distant chunks
#[test]
fn streaming_eviction_ordering() {
    let anchor = coord(0, 0, 0);
    let vy_weight = 2;

    let near_chunk = coord(1, 0, 0); // ring=1
    let far_chunk = coord(10, 0, 0); // ring=10

    // Far chunk should have higher eviction priority (lower key).
    let key_near = EvictionKey::new(near_chunk, anchor, vy_weight, 0);
    let key_far = EvictionKey::new(far_chunk, anchor, vy_weight, 0);

    assert!(key_far < key_near, "far chunk should be evicted before near chunk");

    // Verify chunk states
    let policy = WindowPolicy::default();
    assert_eq!(policy.classify(near_chunk, anchor), ChunkState::Meshed);
    assert_eq!(policy.classify(far_chunk, anchor), ChunkState::Unloaded);
}

/// Test: LOD ring plan composition with streaming window
#[test]
fn lod_ring_plan_composition() {
    let policy = WindowPolicy::default();
    let plan = LodRingPlan::default_for(policy);
    let anchor = coord(0, 0, 0);

    // Inner ring (ring <= mesh_ring)
    assert_eq!(plan.role(coord(0, 0, 0), anchor), RingRole::Inner);
    assert_eq!(plan.role(coord(1, 0, 0), anchor), RingRole::Inner);

    // Seam ring
    assert!(plan.role(coord(2, 0, 0), anchor).is_seam());

    // Frozen (beyond coarse_render_ring)
    assert_eq!(plan.role(coord(5, 0, 0), anchor), RingRole::Frozen);
}
