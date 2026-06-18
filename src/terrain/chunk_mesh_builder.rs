//! Flat + height-mapped chunk mesh builder.
//!
//! Ported from C# `ChunkMeshBuilder.cs`. The `BuildMesh(HeightField, int, float)`
//! overload was a stub that delegated to `BuildMesh(int, float)` and is dropped
//! per the audit (YAGNI).
//!
//! The output `MeshData` uses `glam::Vec3` for positions/normals and
//! `glam::Vec2` for UVs.

use serde::{Deserialize, Serialize};

use crate::terrain::error::{TerrainError, TerrainResult};
use crate::terrain::height_field::HeightField;

/// Vertex + index + UV + normal buffer for a chunk mesh.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeshData {
    /// Vertex positions in world space.
    pub vertices: Vec<glam::Vec3>,
    /// Triangle index buffer (3 indices per triangle, clockwise when viewed from above).
    pub indices: Vec<u32>,
    /// Per-vertex UV coordinates.
    pub uvs: Vec<glam::Vec2>,
    /// Per-vertex normals (default: `Vec3::Y` for flat meshes).
    pub normals: Vec<glam::Vec3>,
}

/// Generates a chunk mesh from a height field (or flat grid).
#[derive(Debug, Default, Clone, Copy)]
pub struct ChunkMeshBuilder;

impl ChunkMeshBuilder {
    /// Build a flat grid mesh at Y = 0.
    ///
    /// `resolution` is the number of quads per side; `size` is the world-space
    /// edge length. The output has `(resolution + 1)²` vertices and
    /// `resolution² * 6` indices.
    pub fn build_mesh(&self, resolution: i32, size: f32) -> TerrainResult<MeshData> {
        if resolution <= 0 {
            return Err(TerrainError::InvalidResolution { value: resolution });
        }
        let res_u = resolution as usize;
        let vertex_count = (res_u + 1) * (res_u + 1);
        let index_count = res_u * res_u * 6;

        let mut vertices = Vec::with_capacity(vertex_count);
        let mut uvs = Vec::with_capacity(vertex_count);
        let mut normals = Vec::with_capacity(vertex_count);

        let cell_size = size / resolution as f32;
        for z in 0..=res_u {
            for x in 0..=res_u {
                vertices.push(glam::Vec3::new(x as f32 * cell_size, 0.0, z as f32 * cell_size));
                uvs.push(glam::Vec2::new(x as f32 / resolution as f32, z as f32 / resolution as f32));
                normals.push(glam::Vec3::Y);
            }
        }

        let mut indices = Vec::with_capacity(index_count);
        for z in 0..res_u {
            for x in 0..res_u {
                let bottom_left = (z * (res_u + 1) + x) as u32;
                let bottom_right = bottom_left + 1;
                let top_left = ((z + 1) * (res_u + 1) + x) as u32;
                let top_right = top_left + 1;
                // First triangle (clockwise from above)
                indices.push(bottom_left);
                indices.push(top_left);
                indices.push(top_right);
                // Second triangle (clockwise from above)
                indices.push(bottom_left);
                indices.push(top_right);
                indices.push(bottom_right);
            }
        }

        Ok(MeshData { vertices, indices, uvs, normals })
    }

    /// Build a height-mapped mesh by sampling `height_field` at each grid vertex.
    /// `height_field` must have dimensions `(resolution + 1) x (resolution + 1)`.
    pub fn build_mesh_from_height(&self, height_field: &HeightField, resolution: i32, size: f32) -> TerrainResult<MeshData> {
        if resolution <= 0 {
            return Err(TerrainError::InvalidResolution { value: resolution });
        }
        let res_u = resolution as usize;
        let vertex_count = (res_u + 1) * (res_u + 1);
        let index_count = res_u * res_u * 6;
        let expected_w = res_u as i32 + 1;
        let expected_h = res_u as i32 + 1;
        if height_field.width() != expected_w || height_field.height() != expected_h {
            return Err(TerrainError::InvalidDataLength {
                got: (height_field.width() * height_field.height()) as usize,
                expected: expected_w as usize * expected_h as usize,
            });
        }

        let mut vertices = Vec::with_capacity(vertex_count);
        let mut uvs = Vec::with_capacity(vertex_count);
        let mut normals = Vec::with_capacity(vertex_count);

        let cell_size = size / resolution as f32;
        for z in 0..=res_u {
            for x in 0..=res_u {
                let y = height_field.get_height(x as i32, z as i32).unwrap_or(0.0);
                vertices.push(glam::Vec3::new(x as f32 * cell_size, y, z as f32 * cell_size));
                uvs.push(glam::Vec2::new(x as f32 / resolution as f32, z as f32 / resolution as f32));
                normals.push(glam::Vec3::Y);
            }
        }

        let mut indices = Vec::with_capacity(index_count);
        for z in 0..res_u {
            for x in 0..res_u {
                let bottom_left = (z * (res_u + 1) + x) as u32;
                let bottom_right = bottom_left + 1;
                let top_left = ((z + 1) * (res_u + 1) + x) as u32;
                let top_right = top_left + 1;
                indices.push(bottom_left);
                indices.push(top_left);
                indices.push(top_right);
                indices.push(bottom_left);
                indices.push(top_right);
                indices.push(bottom_right);
            }
        }

        Ok(MeshData { vertices, indices, uvs, normals })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_mesh_simple_quad_produces_correct_counts() {
        let mesh = ChunkMeshBuilder.build_mesh(1, 1.0).unwrap();
        assert_eq!(mesh.vertices.len(), 4);
        assert_eq!(mesh.uvs.len(), 4);
        assert_eq!(mesh.normals.len(), 4);
        assert_eq!(mesh.indices.len(), 6);
    }

    #[test]
    fn build_mesh_simple_quad_vertices_at_expected_positions() {
        let mesh = ChunkMeshBuilder.build_mesh(1, 2.0).unwrap();
        assert_eq!(mesh.vertices[0], glam::Vec3::new(0.0, 0.0, 0.0));
        assert_eq!(mesh.vertices[1], glam::Vec3::new(2.0, 0.0, 0.0));
        assert_eq!(mesh.vertices[2], glam::Vec3::new(0.0, 0.0, 2.0));
        assert_eq!(mesh.vertices[3], glam::Vec3::new(2.0, 0.0, 2.0));
    }

    #[test]
    fn build_mesh_simple_quad_indices_form_two_triangles() {
        let mesh = ChunkMeshBuilder.build_mesh(1, 1.0).unwrap();
        // First triangle: bottom-left(0), top-left(2), top-right(3)
        assert_eq!(mesh.indices[0], 0);
        assert_eq!(mesh.indices[1], 2);
        assert_eq!(mesh.indices[2], 3);
        // Second triangle: bottom-left(0), top-right(3), bottom-right(1)
        assert_eq!(mesh.indices[3], 0);
        assert_eq!(mesh.indices[4], 3);
        assert_eq!(mesh.indices[5], 1);
    }

    #[test]
    fn build_mesh_triangle_winding_order_is_upward() {
        let mesh = ChunkMeshBuilder.build_mesh(4, 4.0).unwrap();
        let triangle_count = mesh.indices.len() / 3;
        for t in 0..triangle_count {
            let i0 = mesh.indices[t * 3] as usize;
            let i1 = mesh.indices[t * 3 + 1] as usize;
            let i2 = mesh.indices[t * 3 + 2] as usize;
            let v0 = mesh.vertices[i0];
            let v1 = mesh.vertices[i1];
            let v2 = mesh.vertices[i2];
            let edge1 = v1 - v0;
            let edge2 = v2 - v0;
            let normal = edge1.cross(edge2);
            assert!(normal.y > 0.0, "triangle {t} has downward-facing normal");
        }
    }

    #[test]
    fn build_mesh_vertex_count_near_resolution() {
        let mesh = ChunkMeshBuilder.build_mesh(64, 64.0).unwrap();
        // (64+1)² = 4225
        assert_eq!(mesh.vertices.len(), 4225);
    }

    #[test]
    fn build_mesh_vertex_count_mid_resolution() {
        let mesh = ChunkMeshBuilder.build_mesh(32, 64.0).unwrap();
        // (32+1)² = 1089
        assert_eq!(mesh.vertices.len(), 1089);
    }

    #[test]
    fn build_mesh_invalid_resolution() {
        assert!(ChunkMeshBuilder.build_mesh(0, 1.0).is_err());
        assert!(ChunkMeshBuilder.build_mesh(-1, 1.0).is_err());
    }
}
