//! Full scene example: combining terrain, water, and postfx systems.
//!
//! Demonstrates the integration of the `phenotype-gfx` subsystems:
//! voxel terrain meshing, water surface animation, and post-processing.

use anyhow::Result;
use phenotype_gfx::voxel::{Chunk, ChunkId, ChunkView, CHUNK_EDGE, MaterialId};
use phenotype_gfx::voxel::cubic_mesher::CubicMesher;
use phenotype_gfx::voxel::lod::LodLevel;
use phenotype_gfx::voxel::mesh::Mesher;
use phenotype_gfx::water::gerstner_wave_bank::GerstnerWaveBank;
use phenotype_gfx::postfx::{PostStack, PostStackConfig};
use phenotype_gfx::lod::{MvpResidentConfig, LodRingPlan};
use std::time::Instant;

fn main() -> Result<()> {
    println!("=== Full Scene Demo ===");
    let start = Instant::now();

    // 1. Create a voxel terrain chunk
    let mut chunk: Chunk<MaterialId> = Chunk::default();
    // Populate a 8x8x8 block
    for z in 0..8usize {
        for y in 0..8usize {
            for x in 0..8usize {
                let idx = x + y * CHUNK_EDGE + z * CHUNK_EDGE * CHUNK_EDGE;
                chunk.voxels[idx] = MaterialId(1);
            }
        }
    }

    let view = ChunkView {
        id: ChunkId(0),
        voxels: &chunk.voxels,
    };

    // 2. Mesh the voxel terrain
    let mesher: CubicMesher<MaterialId> = CubicMesher::new();
    let mesh = mesher.mesh_chunk(view, LodLevel(0))?;
    println!("Terrain: {} vertices, {} indices", mesh.vertex_count(), mesh.index_count());

    // 3. Set up water surface
    let water_bank = GerstnerWaveBank::create_ocean_preset();
    let water_disp = water_bank.sample_displacement(glam::Vec2::new(4.0, 4.0), 1.0);
    println!("Water: displacement at center = {:?}", water_disp);

    // 4. Configure post-processing
    let post_config = PostStackConfig {
        enable_bloom: true,
        enable_ssao: true,
        ..PostStackConfig::default()
    };
    let mut postfx = PostStack::new(post_config);
    postfx.validate_shader_variants(&phenotype_gfx::postfx::ports::shader_availability::DefaultPostFxShaderAvailability);
    println!("PostFX: SSAO supported={}, Bloom supported={}", postfx.ssao_supported(), postfx.bloom_supported());

    // 5. System stats / performance metrics
    let mvp = MvpResidentConfig::MVP;
    let ring_plan = LodRingPlan::default();
    println!("MVP world side: {}m", mvp.mvp_world_side_m());
    println!("LOD ring inner chunks per side: {}", ring_plan.inner_side_chunks());

    let elapsed = start.elapsed();
    println!("\nScene initialization completed in {:?}", elapsed);
    println!("Full scene demo completed successfully.");
    Ok(())
}
