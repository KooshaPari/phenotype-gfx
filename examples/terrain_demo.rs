//! Terrain generation example: Perlin-like height field, LOD selection, and mesh building.
//!
//! Demonstrates the `phenotype-gfx` terrain subsystem: height-field construction,
//! LOD-aware mesh generation, and basic camera-plane setup.

use anyhow::Result;
use phenotype_gfx::terrain::chunk_mesh_builder::{ChunkMeshBuilder, MeshData};
use phenotype_gfx::terrain::height_field::HeightField;
use phenotype_gfx::lod::select_mesh_detail_level;
use phenotype_gfx::{LodPolicy, VoxelScaleMultiplier};

/// Simple Perlin-like noise approximation using sine/cosine.
fn pseudo_perlin(x: f32, z: f32) -> f32 {
    let v1 = (x * 0.1 + z * 0.15).sin() * 5.0;
    let v2 = (x * 0.05 - z * 0.08).cos() * 10.0;
    let v3 = (x * 0.2 + z * 0.1).sin() * 2.0;
    v1 + v2 + v3
}

fn main() -> Result<()> {
    println!("=== Terrain Demo ===");

    // 1. Build a 32x32 height field with procedural noise
    let size = 32;
    let mut hf = HeightField::new(size, size)?;
    for z in 0..size {
        for x in 0..size {
            let height = pseudo_perlin(x as f32, z as f32);
            hf.set_height(x, z, height)?;
        }
    }
    println!("Height field created: {}x{}", hf.width(), hf.height());

    // 2. Build a mesh from the height field
    let mesh: MeshData = ChunkMeshBuilder.build_mesh_from_height(&hf, size - 1, 32.0)?;
    println!(
        "Mesh generated: {} vertices, {} indices",
        mesh.vertices.len(),
        mesh.indices.len()
    );

    // 3. Demonstrate LOD selection for different distances
    let scale = VoxelScaleMultiplier(8.0);
    let policy = LodPolicy::default();
    let distances = [5.0, 50.0, 200.0, 1000.0];

    println!("\nLOD Selection:");
    for dist in distances {
        let lod = select_mesh_detail_level(dist, scale, policy);
        println!("  Distance {dist:.1}m -> LOD Level {}", lod.0);
    }

    // 4. Basic camera setup simulation
    let camera_pos = glam::Vec3::new(16.0, 20.0, 16.0);
    let camera_target = glam::Vec3::new(16.0, 0.0, 16.0);
    let forward = (camera_target - camera_pos).normalize();
    println!("\nCamera at {camera_pos:?} looking towards {forward:?}");

    println!("\nTerrain demo completed successfully.");
    Ok(())
}
