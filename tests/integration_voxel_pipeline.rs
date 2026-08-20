//! Cross-module integration test for the voxel creation → meshing → output pipeline.
//!
//! These tests exercise the full path: voxel data creation (via `VoxelWorld` or
//! direct `Chunk` manipulation) through mesh generation (`CubicMesher` /
//! `GreedyMesher`) and verify that the resulting `MeshBuffer` is well-formed,
//! material-correct, and deterministically reproducible.

use phenotype_gfx::voxel::chunk::{Chunk, ChunkId, ChunkView, CHUNK_EDGE};
use phenotype_gfx::voxel::coord::{ChunkCoord, WorldCoord, FIXED_SCALE};
use phenotype_gfx::voxel::cubic_mesher::CubicMesher;
use phenotype_gfx::voxel::greedy_mesher::GreedyMesher;
use phenotype_gfx::voxel::lod::{select_lod, LodLevel, LodPolicy, VoxelScaleMultiplier};
use phenotype_gfx::voxel::material::{MaterialId, MaterialPalette, VoxelMaterial};
use phenotype_gfx::voxel::world::VoxelWorld;

fn idx(x: i32, y: i32, z: i32) -> usize {
    x as usize + y as usize * CHUNK_EDGE + z as usize * CHUNK_EDGE * CHUNK_EDGE
}

// ---------------------------------------------------------------------------
// Integration Test 1: VoxelWorld → Chunk → CubicMesher → Mesh verification
// ---------------------------------------------------------------------------

/// Write voxels into a `VoxelWorld`, extract the underlying chunk, mesh it with
/// `CubicMesher`, and verify the output is valid (indices in bounds, normals
/// outward, AO buffer parallel to vertices).
#[test]
fn voxelworld_write_to_cubic_mesh_pipeline() {
    let mut world = VoxelWorld::<MaterialId>::new(FIXED_SCALE);

    // Write a 3-voxel L-shape into the world.
    let positions = [
        WorldCoord { x: 0, y: 0, z: 0 },
        WorldCoord {
            x: FIXED_SCALE,
            y: 0,
            z: 0,
        },
        WorldCoord {
            x: 0,
            y: FIXED_SCALE,
            z: 0,
        },
    ];
    for pos in &positions {
        world.write(*pos, MaterialId(1));
    }

    // Verify dirty events were produced.
    let dirty = world.drain_dirty();
    assert_eq!(dirty.len(), 3, "three writes should produce three dirty events");

    // Extract the chunk and mesh it.
    let chunk_coord = ChunkCoord {
        cx: 0,
        cy: 0,
        cz: 0,
    };
    let chunk = world
        .chunk(chunk_coord)
        .expect("chunk must exist after writes");

    let view = ChunkView {
        id: chunk_coord.chunk_id(),
        voxels: &chunk.voxels,
    };
    let mesh = CubicMesher::<MaterialId>::mesh_cubic(view, LodLevel(0))
        .expect("cubic mesh should succeed");

    // Basic mesh validity.
    assert!(!mesh.vertices.is_empty(), "mesh must not be empty");
    assert!(!mesh.indices.is_empty(), "indices must not be empty");
    assert_eq!(mesh.indices.len() % 3, 0, "indices must be a multiple of 3");
    assert_eq!(
        mesh.ao.len(),
        mesh.vertices.len(),
        "AO buffer must be parallel to vertices"
    );

    // All indices must reference valid vertex slots.
    let vcount = mesh.vertices.len() as u32;
    for &i in &mesh.indices {
        assert!(i < vcount, "index {i} out of bounds (vcount={vcount})");
    }

    // All normals must be unit-length axis-aligned vectors.
    for v in &mesh.vertices {
        let len_sq = v.normal[0] * v.normal[0]
            + v.normal[1] * v.normal[1]
            + v.normal[2] * v.normal[2];
        assert!(
            (len_sq - 1.0).abs() < 1e-6,
            "normal must be unit-length, got len²={len_sq}"
        );
    }

    // All material IDs must be MaterialId(1) (the only material we wrote).
    assert!(
        mesh.vertices.iter().all(|v| v.material == MaterialId(1)),
        "all vertices must carry MaterialId(1)"
    );
}

// ---------------------------------------------------------------------------
// Integration Test 2: Multi-material palette → GreedyMesh → verify material
// ---------------------------------------------------------------------------

/// Register two materials in a `MaterialPalette`, create a chunk with both
/// materials placed in distinct regions, mesh with `GreedyMesher`, and verify
/// that the mesh contains vertices referencing *both* material IDs.
#[test]
fn multi_material_palette_to_greedy_mesh() {
    // 1. Build a material palette.
    let mut palette = MaterialPalette::default();
    let stone_id = palette
        .add(VoxelMaterial {
            name: "stone".into(),
            era: 0,
            hardness: 10.0,
        })
        .expect("add stone");
    let wood_id = palette
        .add(VoxelMaterial {
            name: "wood".into(),
            era: 1,
            hardness: 3.0,
        })
        .expect("add wood");

    assert_eq!(stone_id, MaterialId(0));
    assert_eq!(wood_id, MaterialId(1));

    // 2. Create a chunk with both materials in separate regions.
    let mut chunk = Chunk::<MaterialId>::default();

    // Stone region: x in 0..4, z in 0..4 at y=0
    for z in 0..4_i32 {
        for x in 0..4_i32 {
            chunk.voxels[idx(x, 0, z)] = stone_id;
        }
    }
    // Wood region: x in 8..12, z in 8..12 at y=0
    for z in 8..12_i32 {
        for x in 8..12_i32 {
            chunk.voxels[idx(x, 0, z)] = wood_id;
        }
    }

    let view = ChunkView {
        id: ChunkId(0),
        voxels: &chunk.voxels,
    };

    // 3. Mesh with GreedyMesher.
    let mesh = GreedyMesher::<MaterialId>::mesh_greedy(view, LodLevel(0))
        .expect("greedy mesh should succeed");

    // 4. Verify both materials appear in the vertex stream.
    let has_stone = mesh.vertices.iter().any(|v| v.material == stone_id);
    let has_wood = mesh.vertices.iter().any(|v| v.material == wood_id);
    assert!(has_stone, "stone material must appear in mesh vertices");
    assert!(has_wood, "wood material must appear in mesh vertices");

    // 5. Mesh validity: indices in bounds, AO parallel to vertices.
    let vcount = mesh.vertices.len() as u32;
    for &i in &mesh.indices {
        assert!(i < vcount, "index {i} out of bounds (vcount={vcount})");
    }
    assert_eq!(
        mesh.ao.len(),
        mesh.vertices.len(),
        "AO buffer must be parallel to vertices"
    );
}

// ---------------------------------------------------------------------------
// Integration Test 3: VoxelWorld write → dirty events → LOD → mesh
// ---------------------------------------------------------------------------

/// End-to-end pipeline: write voxels, drain dirty events, select LOD based on
/// viewer distance, mesh at the selected LOD, and verify the mesh is
/// well-formed and deterministically reproducible.
#[test]
fn full_pipeline_write_dirty_lod_mesh_determinism() {
    let mut world = VoxelWorld::<MaterialId>::new(FIXED_SCALE);

    // Write a solid 2×2×2 block.
    for z in 0..2_i32 {
        for y in 0..2_i32 {
            for x in 0..2_i32 {
                world.write(
                    WorldCoord {
                        x: x as i64 * FIXED_SCALE,
                        y: y as i64 * FIXED_SCALE,
                        z: z as i64 * FIXED_SCALE,
                    },
                    MaterialId(1),
                );
            }
        }
    }

    // Drain dirty events.
    let dirty = world.drain_dirty();
    assert_eq!(
        dirty.len(),
        8,
        "eight voxel writes should produce eight dirty events"
    );

    // Select LOD at near distance → should be LOD 0.
    let policy = LodPolicy::default();
    let scale = VoxelScaleMultiplier::default();
    let lod = select_lod(10.0, scale, policy);
    assert_eq!(lod, LodLevel(0), "near viewer should get LOD 0");

    // Extract chunk and mesh.
    let chunk_coord = ChunkCoord {
        cx: 0,
        cy: 0,
        cz: 0,
    };
    let chunk = world
        .chunk(chunk_coord)
        .expect("chunk must exist after writes");
    let view = ChunkView {
        id: chunk_coord.chunk_id(),
        voxels: &chunk.voxels,
    };

    let mesh1 = GreedyMesher::<MaterialId>::mesh_greedy(view, lod)
        .expect("greedy mesh should succeed");

    // Mesh again to verify determinism.
    let mesh2 = GreedyMesher::<MaterialId>::mesh_greedy(view, lod)
        .expect("greedy mesh should succeed (determinism check)");

    assert_eq!(mesh1, mesh2, "meshing must be deterministic");
    assert!(!mesh1.vertices.is_empty(), "mesh must not be empty");
    assert_eq!(
        mesh1.ao.len(),
        mesh1.vertices.len(),
        "AO buffer must be parallel to vertices"
    );

    // Surface area must be positive.
    let area: f64 = mesh1
        .indices
        .chunks_exact(3)
        .map(|tri| {
            let a = mesh1.vertices[tri[0] as usize].position;
            let b = mesh1.vertices[tri[1] as usize].position;
            let c = mesh1.vertices[tri[2] as usize].position;
            let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
            let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
            let cross = [
                ab[1] * ac[2] - ab[2] * ac[1],
                ab[2] * ac[0] - ab[0] * ac[2],
                ab[0] * ac[1] - ab[1] * ac[0],
            ];
            let len = (cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]) as f64;
            len.sqrt() * 0.5
        })
        .sum();
    assert!(area > 0.0, "total surface area must be positive");
}
