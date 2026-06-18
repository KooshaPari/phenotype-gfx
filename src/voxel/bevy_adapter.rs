//! Optional Bevy mesh adapter.
//!
//! This keeps the core crate engine-neutral by default. Consumers that enable the
//! `bevy` feature can turn a [`MeshBuffer`](crate::voxel::mesh::MeshBuffer) into a Bevy
//! [`Mesh`](bevy::render::mesh::Mesh) without re-implementing the buffer mapping.

use crate::voxel::mesh::MeshBuffer;

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, Mesh, MeshVertexAttribute, PrimitiveTopology, VertexFormat};

/// Custom per-vertex ambient-occlusion attribute stored alongside Bevy's built-in
/// position / normal / UV attributes.
pub const ATTRIBUTE_AO: MeshVertexAttribute =
    MeshVertexAttribute::new("VoxelAO", 0x564F_5845_4C5F_414F, VertexFormat::Float32);

/// Convert an engine-neutral [`MeshBuffer`] into a Bevy [`Mesh`].
pub fn to_bevy_mesh(buffer: &MeshBuffer) -> Mesh {
    let positions: Vec<[f32; 3]> = buffer.positions().collect();
    let normals: Vec<[f32; 3]> = buffer.normals().collect();
    let uvs: Vec<[f32; 2]> = buffer.uvs().collect();
    let ao: Vec<f32> = buffer.ao().iter().copied().map(f32::from).collect();

    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_attribute(ATTRIBUTE_AO, ao)
    .with_inserted_indices(Indices::U32(buffer.indices().to_vec()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_mesh_buffer() -> MeshBuffer {
        MeshBuffer {
            vertices: vec![
                crate::voxel::mesh::MeshVertex {
                    position: [0.0, 0.0, 0.0],
                    normal: [0.0, 0.0, 1.0],
                    uv: [0.0, 0.0],
                    material: crate::voxel::material::MaterialId(1),
                },
                crate::voxel::mesh::MeshVertex {
                    position: [1.0, 0.0, 0.0],
                    normal: [0.0, 0.0, 1.0],
                    uv: [1.0, 0.0],
                    material: crate::voxel::material::MaterialId(1),
                },
                crate::voxel::mesh::MeshVertex {
                    position: [1.0, 1.0, 0.0],
                    normal: [0.0, 0.0, 1.0],
                    uv: [1.0, 1.0],
                    material: crate::voxel::material::MaterialId(1),
                },
                crate::voxel::mesh::MeshVertex {
                    position: [0.0, 1.0, 0.0],
                    normal: [0.0, 0.0, 1.0],
                    uv: [0.0, 1.0],
                    material: crate::voxel::material::MaterialId(1),
                },
            ],
            indices: vec![0, 1, 2, 0, 2, 3],
            ao: vec![3, 2, 1, 0],
        }
    }

    /// FR-PHENO-VOXEL-BEVY-000 — Bevy adapter preserves counts and registers attributes.
    #[test]
    fn converts_meshbuffer_into_bevy_mesh() {
        let buffer = sample_mesh_buffer();
        let mesh = to_bevy_mesh(&buffer);

        assert_eq!(mesh.count_vertices(), buffer.vertex_count());
        assert_eq!(
            mesh.indices().map(|indices| indices.len()),
            Some(buffer.index_count())
        );
        assert!(mesh.contains_attribute(Mesh::ATTRIBUTE_POSITION));
        assert!(mesh.contains_attribute(Mesh::ATTRIBUTE_NORMAL));
        assert!(mesh.contains_attribute(Mesh::ATTRIBUTE_UV_0));
        assert!(mesh.contains_attribute(ATTRIBUTE_AO));
    }
}
