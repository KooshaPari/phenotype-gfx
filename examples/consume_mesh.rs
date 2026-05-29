//! Runnable documentation of the engine-agnostic `MeshBuffer` export surface.
//!
//! This example shows the complete consumer contract:
//!   1. Build a chunk from a `MaterialId` palette.
//!   2. Mesh it with `CubicMesher`.
//!   3. Read back vertex/triangle counts and the interleaved GPU buffer.
//!
//! # Interleaved vertex layout (stride = 9 × f32)
//!
//! | offset | field         |
//! |--------|--------------|
//! | 0–2    | position xyz |
//! | 3–5    | normal xyz   |
//! | 6–7    | uv (u, v)    |
//! | 8      | ao (0.0–3.0) |

use phenotype_voxel::{
    chunk::{Chunk, ChunkId, ChunkView, CHUNK_EDGE},
    cubic_mesher::CubicMesher,
    lod::LodLevel,
    material::MaterialId,
    mesh::Mesher,
};

fn main() {
    // --- 1. Build a simple chunk: solid 2×2×2 block in the corner, rest air ---
    let mut chunk: Chunk<MaterialId> = Chunk::default();
    for z in 0..2usize {
        for y in 0..2usize {
            for x in 0..2usize {
                let idx = x + y * CHUNK_EDGE + z * CHUNK_EDGE * CHUNK_EDGE;
                chunk.voxels[idx] = MaterialId(1); // any non-zero = solid
            }
        }
    }

    let view = ChunkView {
        id: ChunkId(0),
        voxels: &chunk.voxels,
    };

    // --- 2. Mesh with the reference cubic mesher ---
    let mesher: CubicMesher<MaterialId> = CubicMesher::new();
    let buf = mesher
        .mesh_chunk(view, LodLevel(0))
        .expect("meshing failed");

    // --- 3. Read back via the export surface ---
    println!("vertex_count   : {}", buf.vertex_count());
    println!("index_count    : {}", buf.index_count());
    println!("triangle_count : {}", buf.triangle_count());
    println!("is_empty       : {}", buf.is_empty());

    let interleaved = buf.to_interleaved();
    println!(
        "interleaved len: {} (= {} verts × 9 floats)",
        interleaved.len(),
        buf.vertex_count()
    );
    assert_eq!(interleaved.len(), buf.vertex_count() * 9);

    // Spot-check first vertex
    if buf.vertex_count() > 0 {
        let v0_pos = [interleaved[0], interleaved[1], interleaved[2]];
        println!("first vertex position: {v0_pos:?}");
    }

    // Accessor round-trip sanity
    assert_eq!(buf.indices().len(), buf.index_count());
    assert_eq!(buf.ao().len(), buf.vertex_count());
    assert_eq!(buf.positions().count(), buf.vertex_count());
    assert_eq!(buf.normals().count(), buf.vertex_count());
    assert_eq!(buf.uvs().count(), buf.vertex_count());

    println!("All assertions passed — consumer contract verified.");
}
